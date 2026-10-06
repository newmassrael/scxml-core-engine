// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A [`Generator`] that runs Claude Code, headless, to write the model of a work.
//!
//! The client is started once per draft, in a folder of its own that is removed when it is
//! done, and is given:
//!
//! - **no built-in tool at all** (`--tools ""`): no shell, no files, no web. What it can do is
//!   what the SCE authoring server offers it, and of that only [`ALLOWED_TOOLS`]: it can read
//!   the work it was started for and check what it writes, and it cannot save to any work, take
//!   or finish a request, or record an acceptance. The application saves what it answers. A
//!   specification is text the owner may have pasted from anywhere, and a client that has read
//!   it should not be one that can be talked into writing to the owner's other work;
//! - **nothing from the machine's settings** (`--setting-sources ""`, `--strict-mcp-config`,
//!   `--disable-slash-commands`): no hook, no `CLAUDE.md`, no other server;
//! - **an answer it must give in one form** (`--json-schema`): the documents of the model and the
//!   requirement list, exactly as it checked them, which is what [`Draft`] is.
//!
//! What it is told goes in on standard input and not on the command line, so that it is not in a
//! process listing; the specification itself is not in it at all, because the client reads the
//! work with `works_read`, which also gives it the owner's answers and the decision record they
//! make. Everything it says comes back as one JSON result; nothing is read from a file it wrote.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

use serde_json::{json, Value};

use crate::auth_policy::Observed;
use crate::client_run::{
    blank_job, capture, draft_from, prompt, schema, supervise, tail, Ended, Scratch, SERVER,
    SYSTEM_PROMPT,
};
use crate::revision::Revision;
use crate::runner::{Cancel, Draft, GenerateError, Generator, Job};

/// The tools the client may use, of the authoring server's. Reading a work and checking a draft;
/// not saving, not taking a request, not accepting. Everything not named here is refused.
pub const ALLOWED_TOOLS: [&str; 9] = [
    "mcp__sce-author__works_read",
    "mcp__sce-author__scxml_kinds",
    "mcp__sce-author__validate_scxml",
    "mcp__sce-author__validate_scxml_set",
    "mcp__sce-author__scxml_unresolved",
    "mcp__sce-author__decisions",
    "mcp__sce-author__scxml_requirement_set",
    "mcp__sce-author__scxml_requirements",
    "mcp__sce-author__render_scxml_pseudocode",
];

/// How the client reaches the SCE authoring server: the host knows where it is installed (a
/// checkout runs it out of the tree, a bundle ships a launcher), and this crate does not.
#[derive(Debug, Clone)]
pub struct AuthorServer {
    pub command: PathBuf,
    pub args: Vec<String>,
    /// What the server needs to find the works folder and the product (`SCE_WORK`,
    /// `SCE_WORKS_DIR`, `SCE_CODEGEN`, `PYTHONPATH`).
    pub env: Vec<(String, String)>,
}

/// How a run of the client is bounded.
#[derive(Debug, Clone)]
pub struct ClaudeCodeConfig {
    /// A model name or alias; the client's own default when absent.
    pub model: Option<String>,
    /// How many turns the client may take before it is stopped.
    pub max_turns: u32,
    /// What a run may cost before it is stopped; unbounded when absent.
    pub max_budget_usd: Option<f64>,
    /// How long a run may take before it is killed.
    pub timeout: Duration,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        ClaudeCodeConfig {
            model: None,
            max_turns: 60,
            max_budget_usd: None,
            timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// Claude Code, as a generator.
#[derive(Debug, Clone)]
pub struct ClaudeCode {
    binary: PathBuf,
    author: AuthorServer,
    config: ClaudeCodeConfig,
    version: Option<String>,
}

impl ClaudeCode {
    /// A generator that runs `binary`. Its version is asked once, now: a client that will not
    /// say it (it is not there, or it is not Claude Code) is a generator with no version, and
    /// the failure comes when it is asked to write, in words about what was not found.
    pub fn new(binary: PathBuf, author: AuthorServer, config: ClaudeCodeConfig) -> Self {
        let version = version_of(&binary);
        ClaudeCode {
            binary,
            author,
            config,
            version,
        }
    }

    /// The first Claude Code the application finds ([`candidates`]): the search path's, then the
    /// installer's folders, so that a window started from a menu finds what a shell would.
    pub fn find(author: AuthorServer, config: ClaudeCodeConfig) -> Option<Self> {
        candidates(&Search::from_environment())
            .into_iter()
            .next()
            .map(|found| ClaudeCode::new(found.path, author, config))
            .filter(|found| found.version.is_some())
    }

    /// How a run of this client is bounded.
    pub fn config(&self) -> &ClaudeCodeConfig {
        &self.config
    }

    /// The program this runs.
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// How the client reaches the authoring server.
    pub fn author(&self) -> &AuthorServer {
        &self.author
    }

    fn command(&self, scratch: &Path) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .current_dir(scratch)
            .arg("-p")
            .args(["--output-format", "json"])
            .args(["--json-schema", &schema().to_string()])
            .args(["--tools", ""])
            .arg("--strict-mcp-config")
            .args([
                "--mcp-config",
                &scratch.join(MCP_FILE).display().to_string(),
            ])
            .args(["--allowedTools", &ALLOWED_TOOLS.join(",")])
            .args(["--permission-mode", "dontAsk"])
            .arg("--no-session-persistence")
            .args(["--setting-sources", ""])
            .arg("--disable-slash-commands")
            .args(["--max-turns", &self.config.max_turns.to_string()])
            .args(["--append-system-prompt", SYSTEM_PROMPT]);
        if let Some(model) = &self.config.model {
            command.args(["--model", model]);
        }
        if let Some(budget) = self.config.max_budget_usd {
            command.args(["--max-budget-usd", &budget.to_string()]);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

/// The file the client reads its servers from, in its folder.
const MCP_FILE: &str = "mcp.json";

impl Generator for ClaudeCode {
    fn kind(&self) -> &str {
        "claude-code"
    }

    fn version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Named by what the client is told and allowed: the system prompt, the wording of the
    /// task (as it reads for a work with nothing in it), the form it must answer in, and the
    /// tools it may use. A change to any of them is another version, with no one to remember
    /// to say so.
    fn instructions(&self) -> Option<String> {
        let material = format!(
            "{SYSTEM_PROMPT}\n{}\n{}\n{}",
            prompt(&blank_job()),
            schema(),
            ALLOWED_TOOLS.join(",")
        );
        let digest = Revision::of(material.as_bytes()).to_string();
        Some(format!("claude-code/{}", &digest[..12]))
    }

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        let scratch = Scratch::new("sce-claude")
            .map_err(|e| GenerateError::Failed(format!("a folder for the client: {e}")))?;
        fs::write(
            scratch.path().join(MCP_FILE),
            mcp_config(&self.author).to_string(),
        )
        .map_err(|e| GenerateError::Failed(format!("the client's server list: {e}")))?;

        let child = self.command(scratch.path()).spawn().map_err(|e| {
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
                "the client took longer than {} minute(s) and was stopped",
                self.config.timeout.as_secs().div_ceil(60).max(1)
            ))),
            Ended::Exited {
                status,
                stdout,
                stderr,
            } => read_answer(status, &stdout, &stderr),
        }
    }
}

/// What the client printed, as a draft or as the reason there is none.
fn read_answer(status: ExitStatus, stdout: &str, stderr: &str) -> Result<Draft, GenerateError> {
    if !status.success() {
        let said = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };
        return Err(GenerateError::Failed(format!(
            "the client stopped with {status}: {}",
            tail(said.trim(), 2000)
        )));
    }
    let result: Value = serde_json::from_str(stdout.trim()).map_err(|_| {
        GenerateError::Failed(format!(
            "the client answered something that is not JSON: {}",
            tail(stdout.trim(), 2000)
        ))
    })?;
    let subtype = result["subtype"].as_str().unwrap_or("unknown");
    if result["is_error"].as_bool().unwrap_or(false) || subtype != "success" {
        return Err(GenerateError::Failed(format!(
            "the client reported {subtype}: {}",
            tail(result["result"].as_str().unwrap_or("").trim(), 2000)
        )));
    }
    // The answer the schema asked for. A client that did not use the schema's channel may still
    // have answered in the form, as the text of its reply.
    let answer = result
        .get("structured_output")
        .filter(|value| !value.is_null())
        .cloned()
        .or_else(|| {
            result["result"]
                .as_str()
                .and_then(|text| serde_json::from_str(text.trim()).ok())
        })
        .ok_or_else(|| {
            GenerateError::Unusable(
                "the reply is not the draft asked for: it has no structured answer".to_string(),
            )
        })?;
    draft_from(&answer).map_err(GenerateError::Unusable)
}

/// The servers the client may reach: the SCE authoring server, and no other.
fn mcp_config(author: &AuthorServer) -> Value {
    let env: serde_json::Map<String, Value> = author
        .env
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect();
    json!({
        "mcpServers": {
            SERVER: {
                "command": author.command.display().to_string(),
                "args": author.args,
                "env": env,
            }
        }
    })
}

/// What the client's program is called on this system.
const PROGRAM: &str = if cfg!(windows) {
    "claude.exe"
} else {
    "claude"
};

/// How long a program that may be Claude Code is given to say what it is.
pub(crate) const SAY_WHAT_IT_IS: Duration = Duration::from_secs(15);

/// Where to look for the client: the search path first, then the places the official installer
/// puts it. A window started from a menu does not have what a shell's startup files add to the
/// path (the installer's folder is one), so a person who has the client installed would be told
/// there is none.
#[derive(Debug, Clone)]
pub struct Search {
    /// The folders of the search path, in its order.
    pub path: Vec<PathBuf>,
    /// The folders it is installed in, which the search path may not name.
    pub known: Vec<PathBuf>,
    /// How long each program is given to say what it is.
    pub timeout: Duration,
}

impl Search {
    /// Where this process can look: its search path, and the installer's folders under the
    /// person's home.
    pub fn from_environment() -> Self {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .filter(|home| !home.is_empty())
            .map(PathBuf::from);
        Search {
            path: std::env::var_os("PATH")
                .map(|path| std::env::split_paths(&path).collect())
                .unwrap_or_default(),
            known: known_directories(home.as_deref()),
            timeout: SAY_WHAT_IT_IS,
        }
    }
}

/// The folders the client is installed in apart from the search path: under the person's home
/// where the official installer puts it, and the system's own places. Without a home folder
/// there are only the ones that need none: a home is not guessed at.
pub fn known_directories(home: Option<&Path>) -> Vec<PathBuf> {
    let mut known = Vec::new();
    if let Some(home) = home {
        known.push(home.join(".local").join("bin"));
        known.push(home.join(".claude").join("local"));
    }
    if cfg!(not(windows)) {
        known.push(PathBuf::from("/usr/local/bin"));
        known.push(PathBuf::from("/opt/homebrew/bin"));
    }
    known
}

/// Where a candidate was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Found {
    SearchPath,
    KnownLocation,
}

/// A program the application found that says it is Claude Code: what it may be told to run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub version: String,
    pub found: Found,
}

/// Whether `path` is a file that can be run.
fn is_runnable(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// What `binary --version` says when it says it is Claude Code (`2.1.291 (Claude Code)`): the
/// version. A program that answers to `claude` and says anything else is not this client, and
/// is not offered.
pub(crate) fn claude_version_of(binary: &Path, timeout: Duration) -> Option<String> {
    let mut command = Command::new(binary);
    command.arg("--version");
    let said = capture(command, timeout).ok()?;
    if !said.status.success() {
        return None;
    }
    let first = said.stdout.lines().next()?.trim();
    if !first.contains("Claude Code") {
        return None;
    }
    first.split_whitespace().next().map(str::to_string)
}

/// The programs in `search` that say they are Claude Code, in the order they were found: the
/// search path's, then the known locations'. One program reached by two names is offered once,
/// by the first; a folder that is not there, a file that cannot be run, and a program that does
/// not answer in time are passed over and do not stop the search.
pub fn candidates(search: &Search) -> Vec<Candidate> {
    let places = search
        .path
        .iter()
        .map(|dir| (dir, Found::SearchPath))
        .chain(search.known.iter().map(|dir| (dir, Found::KnownLocation)));
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut offered = Vec::new();
    for (dir, found) in places {
        let path = dir.join(PROGRAM);
        if !is_runnable(&path) {
            continue;
        }
        // Asked once whatever it is called: a program that is not Claude Code is not asked twice.
        let Ok(real) = fs::canonicalize(&path) else {
            continue;
        };
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);
        if let Some(version) = claude_version_of(&path, search.timeout) {
            offered.push(Candidate {
                path,
                version,
                found,
            });
        }
    }
    offered
}

/// The program that is Claude Code on this computer: the one the environment named (`SCE_CLAUDE`),
/// whatever it is, so that a wrong one is said to be wrong and not quietly replaced; or else the
/// first `search` finds ([`candidates`]). `None` is no program to ask.
pub fn locate(named: Option<&Path>, search: &Search) -> Option<PathBuf> {
    match named {
        Some(path) => Some(path.to_path_buf()),
        None => candidates(search)
            .into_iter()
            .next()
            .map(|found| found.path),
    }
}

/// Who is signed in to the client, asked the way a generation is run: without the machine's
/// settings (`--setting-sources ""`). A login that a settings file supplies (a key it names) is
/// one a generation cannot see, so it is not one the screen may show: asked with the machine's
/// settings, the client says it is signed in by a key a generation would never use.
///
/// `Ok(None)` is nobody signed in. The client fails when nobody is, and still says so in JSON,
/// so what it printed is read before how it ended is looked at.
pub fn observe_auth(binary: &Path) -> Result<Option<Observed>, String> {
    observe_account(binary).map(|account| account.observed)
}

/// The same, with the name of the variable that decided it when the client said one did.
pub fn observe_account(binary: &Path) -> Result<Account, String> {
    let mut command = Command::new(binary);
    command
        .args(["--setting-sources", ""])
        .args(["auth", "status", "--json"]);
    let said = capture(command, Duration::from_secs(20))?;
    account_from_status(said.stdout.trim()).map_err(|e| {
        let stderr = said.stderr.trim();
        if stderr.is_empty() {
            e
        } else {
            format!("{e} ({})", tail(stderr, 300))
        }
    })
}

/// What `claude auth status --json` said of the credential the client is using.
///
/// Only what a screen may show is kept: the client's answer carries the person's email and
/// organization, and neither is read into this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The kind of credential, or `None` for nobody.
    pub observed: Option<Observed>,
    /// The variable that decided it, when the client said one did (a key from the environment, a
    /// cloud provider chosen by one): a name, never a value.
    pub environment: Option<String>,
}

/// The variables a client is told which credential to use by, as far as a screen names them: a
/// source the client reports that is anything else (a helper program, a stored key, a path) is
/// not a variable's name and is not shown.
fn credential_variable(source: &str) -> Option<&str> {
    let named = source.starts_with("ANTHROPIC_")
        && source.len() > "ANTHROPIC_".len()
        && source
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    named.then_some(source)
}

/// What `claude auth status --json` said, as the kind of credential the client is using, or
/// `None` for nobody. A cloud provider is known by the provider and not by the login the client
/// tracks (which is the claude.ai one only); a method the table does not name is [`Observed::Other`]
/// and is not guessed at.
pub fn account_from_status(text: &str) -> Result<Account, String> {
    let status: Value = serde_json::from_str(text).map_err(|_| {
        format!(
            "`claude auth status` did not answer JSON: {}",
            tail(text, 200)
        )
    })?;
    let logged_in = status["loggedIn"]
        .as_bool()
        .ok_or("`claude auth status` did not say whether anybody is signed in")?;
    let provider = status["apiProvider"].as_str().unwrap_or("firstParty");
    let selected_by = match provider {
        "bedrock" => Some("CLAUDE_CODE_USE_BEDROCK"),
        "vertex" => Some("CLAUDE_CODE_USE_VERTEX"),
        "foundry" => Some("CLAUDE_CODE_USE_FOUNDRY"),
        _ => None,
    };
    if provider != "firstParty" {
        return Ok(Account {
            observed: Some(Observed::CloudProvider),
            environment: selected_by.map(str::to_string),
        });
    }
    if !logged_in {
        return Ok(Account {
            observed: None,
            environment: None,
        });
    }
    let observed = match status["authMethod"].as_str() {
        Some("claude.ai") => Observed::Subscription,
        Some("api_key" | "api_key_helper") => Observed::ApiKey,
        _ => Observed::Other,
    };
    Ok(Account {
        observed: Some(observed),
        environment: status["apiKeySource"]
            .as_str()
            .and_then(credential_variable)
            .map(str::to_string),
    })
}

/// What `claude auth status --json` said, as the kind of credential only.
pub fn observed_from_status(text: &str) -> Result<Option<Observed>, String> {
    account_from_status(text).map(|account| account.observed)
}

/// What `binary --version` says, as the version: `2.1.289 (Claude Code)` is `2.1.289`.
pub fn version_of(binary: &Path) -> Option<String> {
    let child = Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    match supervise(child, None, &Cancel::new(), Duration::from_secs(15)).ok()? {
        Ended::Exited { status, stdout, .. } if status.success() => {
            stdout.split_whitespace().next().map(str::to_string)
        }
        _ => None,
    }
}
