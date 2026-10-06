// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A [`Generator`] that runs Codex, headless, to write the model of a work.
//!
//! The client is started once per draft with `codex exec`, in a folder of its own that is removed
//! when it is done, and is given:
//!
//! - **the authoring server and nothing else to reach**, and of its tools only [`AUTHOR_TOOLS`]:
//!   it can read the work it was started for and check what it writes, and it cannot save to any
//!   work, take or finish a request, or record an acceptance. The application saves what it
//!   answers;
//! - **a sandbox that does not write** (`-s read-only`), no question to a person who is not
//!   there (`approval_policy="never"`), no web search, nothing kept (`--ephemeral`) and nothing
//!   read from the person's own settings (`--ignore-user-config`);
//! - **the built-in features switched off that the support list says to switch off** (`--disable`),
//!   which is a list and not a guarantee: Codex has no switch that turns its tools off, and gains
//!   tools from version to version. That is why a version is run only when a person verified it
//!   against a specification written to attack it, and a feature that is on and that nobody
//!   switched off or reviewed is a refusal ([`crate::codex_support`]);
//! - **one credential**, the one the connection chose, and none of the others
//!   ([`crate::codex_environment`]);
//! - **an answer it must give in one form** (`--output-schema`), the same as the other client's:
//!   the documents of the model and the requirement list, which is what [`Draft`] is.
//!
//! What it is told goes in on standard input and not on the command line; the specification itself
//! is not in it at all, because the client reads the work through the authoring server. What it
//! answers is read from the file `-o` names, the last message it wrote, and nothing else it prints.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::auth_policy::Observed;
use crate::claude_code::AuthorServer;
use crate::client_run::{
    blank_job, capture, draft_from, prompt, schema, span_words, supervise, tail, Ended, Scratch,
    SERVER, SYSTEM_PROMPT,
};
use crate::codex_environment::environment_for;
use crate::codex_support::{enabled_features, Support};
use crate::connection::AuthSource;
use crate::revision::Revision;
use crate::runner::{Cancel, Draft, GenerateError, Generator, Job};

/// The authoring server's tools the client may use, by the names the server gives them. Reading
/// a work and checking a draft; not saving, not taking a request, not accepting. This is the
/// list the other client is allowed, in the spelling Codex wants (no server prefix).
pub const AUTHOR_TOOLS: [&str; 9] = [
    "works_read",
    "scxml_kinds",
    "validate_scxml",
    "validate_scxml_set",
    "scxml_unresolved",
    "decisions",
    "scxml_requirement_set",
    "scxml_requirements",
    "render_scxml_pseudocode",
];

/// How long a program that may be Codex is given to say its version or its features.
const SAY: Duration = Duration::from_secs(15);

/// How a run of the client is bounded.
#[derive(Debug, Clone)]
pub struct CodexConfig {
    /// A model name; the client's own default when absent.
    pub model: Option<String>,
    /// How long a run may take before it is killed.
    pub timeout: Duration,
}

impl Default for CodexConfig {
    fn default() -> Self {
        CodexConfig {
            model: None,
            timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// Codex, as a generator.
#[derive(Debug, Clone)]
pub struct Codex {
    binary: PathBuf,
    author: AuthorServer,
    config: CodexConfig,
    auth: AuthSource,
    app_home: PathBuf,
    support: Support,
    version: Option<String>,
    environment: Vec<(String, String)>,
}

impl Codex {
    /// A generator that runs `binary` with the credential `auth` names, `app_home` being the
    /// application's own home for the client. Its version is asked once, now.
    pub fn new(
        binary: PathBuf,
        author: AuthorServer,
        config: CodexConfig,
        auth: AuthSource,
        app_home: PathBuf,
        support: Support,
    ) -> Self {
        let version = version_of(&binary);
        Codex {
            binary,
            author,
            config,
            auth,
            app_home,
            support,
            version,
            environment: std::env::vars().collect(),
        }
    }

    /// The same, started with `environment` as the process's own: what a run is given is worked
    /// out from it, and it is all the client sees of it.
    pub fn with_environment(mut self, environment: Vec<(String, String)>) -> Self {
        self.environment = environment;
        self
    }

    /// The same, bounded as `config` says: what a request pinned of the model and the time, put on
    /// a client that was already asked what it is.
    pub fn with_config(mut self, config: CodexConfig) -> Self {
        self.config = config;
        self
    }

    /// The program this runs.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// The arguments of a run, which are what a person verifies. `scratch` is the folder the run
    /// works in, and `last` the file it writes its last message to.
    fn arguments(&self, scratch: &Path, schema_file: &Path, last: &Path) -> Vec<String> {
        let mut args: Vec<String> = vec!["exec".into(), "--json".into()];
        args.extend(["--output-schema".into(), schema_file.display().to_string()]);
        args.extend(["-o".into(), last.display().to_string()]);
        args.extend([
            "--ephemeral".into(),
            "--ignore-user-config".into(),
            "-s".into(),
            "read-only".into(),
            "--skip-git-repo-check".into(),
            "-C".into(),
            scratch.display().to_string(),
        ]);
        if let Some(model) = &self.config.model {
            args.extend(["-m".into(), model.clone()]);
        }
        for feature in self.support.disabled_features() {
            args.extend(["--disable".into(), feature.clone()]);
        }
        for setting in self.settings() {
            args.extend(["-c".into(), setting]);
        }
        // The prompt is standard input.
        args.push("-".into());
        args
    }

    /// The configuration a run is given on the command line, as `key=value` with a TOML value.
    fn settings(&self) -> Vec<String> {
        let key = |name: &str| format!("mcp_servers.{SERVER}.{name}");
        let env: Vec<String> = self
            .author
            .env
            .iter()
            .map(|(name, value)| format!("{}={}", toml_string(name), toml_string(value)))
            .collect();
        vec![
            "approval_policy=\"never\"".to_string(),
            "web_search=\"disabled\"".to_string(),
            format!(
                "{}={}",
                key("command"),
                toml_string(&self.author.command.display().to_string())
            ),
            format!(
                "{}=[{}]",
                key("args"),
                self.author
                    .args
                    .iter()
                    .map(|a| toml_string(a))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            format!("{}={{{}}}", key("env"), env.join(",")),
            format!(
                "{}=[{}]",
                key("enabled_tools"),
                AUTHOR_TOOLS
                    .iter()
                    .map(|t| toml_string(t))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        ]
    }

    /// The features the installed Codex has switched on, asked now.
    fn enabled(&self) -> Result<Vec<String>, String> {
        let mut command = Command::new(&self.binary);
        command.args(["features", "list"]);
        let said =
            capture(command, SAY).map_err(|e| format!("Codex could not list its features: {e}"))?;
        if !said.status.success() {
            return Err(format!(
                "Codex could not list its features: {}",
                tail(said.stderr.trim(), 500)
            ));
        }
        Ok(enabled_features(&said.stdout))
    }

    /// The environment the client is started in: the process's own, less the credentials that
    /// are not the one chosen, with the home set when the connection has one of its own.
    fn run_environment(&self) -> Result<Run, String> {
        let environment = environment_for(self.auth, &self.app_home, &self.environment)
            .map_err(|e| e.to_string())?;
        let kept = self
            .environment
            .iter()
            .filter(|(name, _)| !environment.remove.contains(name))
            .cloned()
            .collect();
        Ok(Run {
            kept,
            home: environment.home,
        })
    }

    /// Everything that must be true before the client is started, asked in one place so that a run
    /// and the reason a request waits are the same judgement: it is Codex, a version a person
    /// verified, with nothing switched on that nobody looked at, and a credential to give it.
    /// A refusal is a sentence that names no path: it is said where the works folder is.
    fn preflight(&self) -> Result<Run, String> {
        let version = self
            .version
            .as_deref()
            .ok_or("the program did not say it is Codex, so it is not run")?;
        let instructions = self.instructions().unwrap_or_default();
        let enabled = self.enabled()?;
        self.support
            .check(std::env::consts::OS, version, &instructions, &enabled)
            .map_err(|e| e.to_string())?;
        self.run_environment()
    }

    /// Why this Codex cannot be run now, or `None` when it can: for the request that waits.
    pub fn why_not(&self) -> Option<String> {
        self.preflight().err()
    }

    /// Who is signed in to Codex for this connection, asked of the client the way a run
    /// is started (the same credentials, the same home), or why it could not be asked. `None` is
    /// nobody.
    pub fn observe_login(&self) -> Result<Option<Observed>, String> {
        let run = self.run_environment()?;
        let mut command = Command::new(&self.binary);
        command.args(["login", "status"]).env_clear().envs(run.kept);
        if let Some(home) = &run.home {
            command.env("CODEX_HOME", home);
        }
        let said = capture(command, SAY)
            .map_err(|_| "Codex could not be asked who is signed in".to_string())?;
        // It says so on either stream, and fails when nobody is: what it printed is read, not how
        // it ended.
        Ok(observed_from_login_status(&format!(
            "{}\n{}",
            said.stdout, said.stderr
        )))
    }
}

/// What a run is started with.
struct Run {
    /// The process's own environment less the credentials that are not the one chosen.
    kept: Vec<(String, String)>,
    /// The home to give the client, when it is not the one it would use itself.
    home: Option<PathBuf>,
}

/// What `codex login status` said, as the kind of credential in use, or `None` for nobody. A way it
/// does not name is [`Observed::Other`] and is not guessed at.
pub fn observed_from_login_status(text: &str) -> Option<Observed> {
    let said = text.to_ascii_lowercase();
    if said.contains("not logged in") {
        None
    } else if said.contains("chatgpt") {
        Some(Observed::Subscription)
    } else if said.contains("api key") {
        Some(Observed::ApiKey)
    } else {
        Some(Observed::Other)
    }
}

/// The Codex a host found, and how a run of it is given what it needs: what a directory needs to
/// make a generator for a request that names a connection to Codex.
#[derive(Debug, Clone)]
pub struct CodexLaunch {
    pub binary: PathBuf,
    pub author: AuthorServer,
    /// The application's own home for the client, which a connection that chose the application's
    /// stored login uses.
    pub app_home: PathBuf,
    /// Which versions were verified, and what is switched off.
    pub support: Support,
    /// The environment a run is worked out from: the process's own.
    pub environment: Vec<(String, String)>,
}

/// A string as a TOML value: a basic string, whose escapes are JSON's.
fn toml_string(text: &str) -> String {
    serde_json::to_string(text).expect("a string is always JSON")
}

/// What `binary --version` says when it says it is Codex (`codex-cli 0.159.0`): the version.
fn version_of(binary: &Path) -> Option<String> {
    let mut command = Command::new(binary);
    command.arg("--version");
    let said = capture(command, SAY).ok()?;
    if !said.status.success() {
        return None;
    }
    let first = said.stdout.lines().next()?.trim();
    if !first.to_ascii_lowercase().contains("codex") {
        return None;
    }
    first.split_whitespace().last().map(str::to_string)
}

impl Generator for Codex {
    fn kind(&self) -> &str {
        "codex"
    }

    fn version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Named by what the client is told, what it must answer in, which tools it may use, and what
    /// is switched off and reviewed. A change to any of them is another version, and another
    /// entry on the list of what a person verified, with no one to remember to say so.
    fn instructions(&self) -> Option<String> {
        let material = format!(
            "{SYSTEM_PROMPT}\n{}\n{}\n{}\n{}\n{}",
            prompt(&blank_job()),
            schema(),
            AUTHOR_TOOLS.join(","),
            self.support.disabled_features().join(","),
            self.support.reviewed_features().join(","),
        );
        let digest = Revision::of(material.as_bytes()).to_string();
        Some(format!("codex/{}", &digest[..12]))
    }

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        // Nothing is started before it is known to be a Codex this build has verified, with
        // nothing on that nobody looked at, and a credential to give it.
        let Run { kept, home } = self.preflight().map_err(GenerateError::Failed)?;

        let scratch = Scratch::new("sce-codex")
            .map_err(|e| GenerateError::Failed(format!("a folder for the client: {e}")))?;
        let schema_file = scratch.path().join("schema.json");
        let last = scratch.path().join("last.json");
        fs::write(&schema_file, schema().to_string())
            .map_err(|e| GenerateError::Failed(format!("the form of the answer: {e}")))?;

        let mut command = Command::new(&self.binary);
        command
            .current_dir(scratch.path())
            .args(self.arguments(scratch.path(), &schema_file, &last))
            .env_clear()
            .envs(kept)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // The home the connection gave the client, when it gave one.
        if let Some(home) = &home {
            command.env("CODEX_HOME", home);
        }
        let child = command.spawn().map_err(|e| {
            GenerateError::Failed(format!(
                "{} could not be started: {e}",
                self.binary.display()
            ))
        })?;
        let ended = supervise(
            child,
            Some(prompt(job).into_bytes()),
            cancel,
            self.config.timeout,
        )
        .map_err(|e| GenerateError::Failed(format!("the client could not be read: {e}")))?;

        match ended {
            Ended::Cancelled => Err(GenerateError::Cancelled),
            Ended::TimedOut => Err(GenerateError::Failed(format!(
                "the client took longer than {} and was stopped",
                span_words(self.config.timeout)
            ))),
            Ended::Exited {
                status,
                stdout,
                stderr,
            } => {
                if !status.success() {
                    let said = if stderr.trim().is_empty() {
                        &stdout
                    } else {
                        &stderr
                    };
                    return Err(GenerateError::Failed(format!(
                        "the client stopped with {status}: {}",
                        tail(said.trim(), 2000)
                    )));
                }
                read_answer(&last)
            }
        }
    }
}

/// The last message the client wrote, as a draft or as the reason there is none.
fn read_answer(last: &Path) -> Result<Draft, GenerateError> {
    let text = fs::read_to_string(last).map_err(|_| {
        GenerateError::Unusable(
            "the client did not write a last message, so there is no draft".to_string(),
        )
    })?;
    let answer: Value = serde_json::from_str(text.trim()).map_err(|_| {
        GenerateError::Unusable(format!(
            "the last message is not the draft asked for: it is not JSON: {}",
            tail(text.trim(), 2000)
        ))
    })?;
    draft_from(&answer).map_err(GenerateError::Unusable)
}
