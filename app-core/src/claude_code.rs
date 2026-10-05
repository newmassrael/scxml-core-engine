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
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::model_set::{Document, ModelFiles};
use crate::requirements::Requirements;
use crate::runner::{Cancel, Draft, GenerateError, Generator, Job};

/// What the SCE authoring server is called to the client, and so the prefix of its tools.
const SERVER: &str = "sce-author";

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

/// The most of what a client writes that is kept to show the owner when it fails.
const KEPT_BYTES: usize = 16 * 1024 * 1024;

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

    /// The `claude` on the search path, when there is one that answers `--version`.
    pub fn find(author: AuthorServer, config: ClaudeCodeConfig) -> Option<Self> {
        let name = if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        };
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
            .map(|binary| ClaudeCode::new(binary, author, config))
            .filter(|found| found.version.is_some())
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

    fn generate(&self, job: &Job, cancel: &Cancel) -> Result<Draft, GenerateError> {
        let scratch = Scratch::new()
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

/// The documents of the model and the requirement list, as the application keeps them.
fn draft_from(answer: &Value) -> Result<Draft, String> {
    let documents = answer["model"]["documents"]
        .as_array()
        .ok_or("the answer has no `model.documents` list: the model is missing")?;
    let documents: Vec<Document> = documents
        .iter()
        .map(|d| match (d["name"].as_str(), d["text"].as_str()) {
            (Some(name), Some(text)) => Ok(Document {
                name: name.to_string(),
                text: text.to_string(),
            }),
            _ => Err("every entry of `model.documents` has a `name` and a `text`".to_string()),
        })
        .collect::<Result<_, _>>()?;
    let entry = answer["model"]["entry"].as_str();
    let model = match (documents.len(), entry) {
        (0, _) => return Err("`model.documents` is empty: the model has no documents".to_string()),
        // One document is the model as it always was, whatever name it was given.
        (1, None) => ModelFiles::single(documents[0].text.clone()),
        _ => ModelFiles::set(documents, entry).map_err(|e| e.to_string())?,
    };
    let manifest = answer["requirements"]["manifest_text"]
        .as_str()
        .ok_or("the answer has no `requirements.manifest_text`: the requirement list is missing")?
        .to_string();
    let sidecar = answer["requirements"]["sidecar_text"]
        .as_str()
        .map(str::to_string);
    let requirements = Requirements::new(manifest, sidecar)
        .map_err(|e| format!("the requirement list is not usable: its manifest or sidecar: {e}"))?;
    Ok(Draft {
        model,
        requirements,
    })
}

/// The form the client must answer in.
fn schema() -> Value {
    json!({
        "type": "object",
        "required": ["model", "requirements"],
        "properties": {
            "model": {
                "type": "object",
                "required": ["documents"],
                "properties": {
                    "documents": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "required": ["name", "text"],
                            "properties": {
                                "name": {"type": "string"},
                                "text": {"type": "string"},
                            },
                        },
                    },
                    "entry": {"type": "string"},
                },
            },
            "requirements": {
                "type": "object",
                "required": ["manifest_text"],
                "properties": {
                    "manifest_text": {"type": "string"},
                    "sidecar_text": {"type": "string"},
                },
            },
        },
    })
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

/// Said to the client beside what the product already tells it (the authoring server's own
/// instructions). What is different here is that nobody is on the other end.
const SYSTEM_PROMPT: &str = "You are the model-writing step of the SCE specification \
workbench. No person is on the other end of this conversation, so never ask a question: a \
decision the specification does not make is marked in the model as the authoring flow says \
(sce:unresolved), never guessed. You can read the work and check what you write, and you \
cannot save anything: the application saves what you answer, in the form it asks for. Your \
last message is that answer and nothing else.";

/// What the client is asked to do for `job`.
fn prompt(job: &Job) -> String {
    let mut text = format!(
        "Write the model for the work `{work}` (\"{title}\"), for request {request}, attempt \
         {attempt}.\n\n\
         1. Call works_read with work = \"{work}\". It gives the specification (`source.text`), \
            the owner's answers to the questions an earlier model left open (`answers`) and the \
            decision record they make (`decisions_text`), and the model the work has now, if it \
            has one.\n\
         2. Follow the authoring flow in your instructions: choose the document kind from what \
            the specification states (scxml_kinds), write the SCXML, check it with validate_scxml \
            (validate_scxml_set for several documents that import each other), and hold it to \
            the owner's answers with the decisions tool and `decisions_text`. Apply each answer \
            and cite it as sce:assumed=\"<id>\"; never leave an answered question \
            sce:unresolved.\n\
         3. Build the requirement list from `source.text` with scxml_requirement_set, and check \
            the design against it with scxml_requirements.\n\
         4. Do not call any tool that saves, takes a request or accepts: you have none. Your \
            last message is the draft in the form asked for: the model's documents exactly as \
            you checked them, and `manifest_text` and `sidecar_text` exactly as \
            scxml_requirement_set returned them.\n",
        work = job.work.as_str(),
        title = job.title,
        request = job.request,
        attempt = job.attempt,
    );
    if let Some(refusal) = &job.refusal {
        text.push_str(&format!(
            "\nYour last draft was refused. What was said of it:\n{refusal}\n\
             Write the draft again so that each finding is answered, and check it again before \
             you answer.\n"
        ));
    }
    text
}

/// A folder for one run, private to the owner and removed with it.
struct Scratch(PathBuf);

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

impl Scratch {
    fn new() -> io::Result<Scratch> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        for _ in 0..8 {
            let path = std::env::temp_dir().join(format!(
                "sce-claude-{}-{}-{nanos}",
                std::process::id(),
                SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
                    }
                    return Ok(Scratch(path));
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "eight generated folder names were all taken",
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// How a run of the client ended.
enum Ended {
    Exited {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
    Cancelled,
    TimedOut,
}

/// Feed the client its input, read everything it says as it says it, and wait for it to end,
/// to be told to stop, or to run out of time.
///
/// Reading is not left until the client has exited: more than a pipe holds, written to a pipe
/// nobody reads, holds the client at its write for ever and it never exits.
fn supervise(
    mut child: Child,
    input: Option<Vec<u8>>,
    cancel: &Cancel,
    timeout: Duration,
) -> io::Result<Ended> {
    let writer = child.stdin.take().map(|mut stdin| {
        thread::spawn(move || {
            if let Some(input) = input {
                // A client that stops before it has read everything closes the pipe: not an
                // error of ours, and its exit says what happened.
                let _ = stdin.write_all(&input);
            }
        })
    });
    let out = child.stdout.take().map(reader);
    let err = child.stderr.take().map(reader);

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if cancel.is_cancelled() {
            stop(&mut child);
            break None;
        }
        if started.elapsed() > timeout {
            stop(&mut child);
            let _ = join(writer, out, err);
            return Ok(Ended::TimedOut);
        }
        thread::sleep(Duration::from_millis(20));
    };
    let (stdout, stderr) = join(writer, out, err);
    Ok(match status {
        Some(status) => Ended::Exited {
            status,
            stdout,
            stderr,
        },
        None => Ended::Cancelled,
    })
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Read a pipe to its end on a thread of its own; what is kept is the first part of it.
fn reader<R: Read + Send + 'static>(mut pipe: R) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    // Past the limit it is still read, so that the client is never held at a
                    // write, and not kept.
                    if kept.len() < KEPT_BYTES {
                        let room = KEPT_BYTES - kept.len();
                        kept.extend_from_slice(&chunk[..n.min(room)]);
                    }
                }
            }
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

fn join(
    writer: Option<thread::JoinHandle<()>>,
    out: Option<thread::JoinHandle<String>>,
    err: Option<thread::JoinHandle<String>>,
) -> (String, String) {
    if let Some(writer) = writer {
        let _ = writer.join();
    }
    let take = |handle: Option<thread::JoinHandle<String>>| {
        handle
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default()
    };
    (take(out), take(err))
}

/// The last `limit` characters of `text`, for a message that says what the client said.
fn tail(text: &str, limit: usize) -> String {
    let count = text.chars().count();
    if count <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().skip(count - limit).collect();
    format!("...{kept}")
}

/// What `binary --version` says, as the version: `2.1.289 (Claude Code)` is `2.1.289`.
fn version_of(binary: &Path) -> Option<String> {
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
