// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A client of an MCP server that is a program of this installation, spoken to over its standard
//! input and output.
//!
//! A client that brings its own tool loop (a model server that only answers messages) needs what a
//! client such as Claude Code does by itself: to start the authoring server, ask it which tools it
//! has, and call them. That is all of the protocol this needs: `initialize`, `tools/list` and
//! `tools/call`, one JSON document to a line, answered in order.
//!
//! What it holds to:
//!
//! - **A call ends when it is told to or when its time is up.** The server is a process that can
//!   hang, so what it says is read on a thread of its own and waited for in slices, and what is
//!   sent to it is written on another: a server that reads none of it (and a call can be longer
//!   than a pipe holds) holds a write for as long as it lives, and a caller that waited on that
//!   could not be told to stop.
//! - **The server is the application's and nothing else's.** It is started with the arguments and
//!   the environment the host gave it, in a folder of its own, and it is killed when this is
//!   dropped, so a run that ends does not leave one behind.
//! - **What it says of itself is not trusted to be small.** A line and a tool's answer each have a
//!   most.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::claude_code::AuthorServer;
use crate::runner::Cancel;

/// How late a word to stop is noticed while a call is waited for.
const SLICE: Duration = Duration::from_millis(100);

/// The most of one line the server says.
const LINE_MAX: usize = 8 * 1024 * 1024;

/// The most of the server's standard error that is kept to say why it failed.
const STDERR_KEPT: usize = 4096;

/// The protocol revision this asks for. The server answers with the one it speaks; what is used is
/// only what every revision since has kept (`tools/list`, `tools/call`).
const PROTOCOL: &str = "2025-06-18";

/// One tool the server has, as it says it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// The JSON Schema of its arguments.
    pub schema: Value,
}

/// What a call answered: the words it gave, and whether it says it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Called {
    pub text: String,
    pub is_error: bool,
}

/// Why the server did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpError {
    Cancelled,
    TimedOut,
    /// The server could not be started or went away; says what, and what it last said.
    Gone(String),
    /// The server answered with an error of its own.
    Refused(String),
    /// The server said something that is not what the protocol says.
    Protocol(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::Cancelled => f.write_str("it was stopped"),
            McpError::TimedOut => f.write_str("the authoring server did not answer in time"),
            McpError::Gone(why) => write!(f, "the authoring server is not running: {why}"),
            McpError::Refused(why) => write!(f, "the authoring server refused: {why}"),
            McpError::Protocol(why) => {
                write!(f, "the authoring server said something unexpected: {why}")
            }
        }
    }
}

impl std::error::Error for McpError {}

/// A started server, asked for its tools and called.
pub struct McpClient {
    child: Child,
    /// What is to be written to the server's standard input. It is written on a thread of its own:
    /// a server that does not read what it is sent holds a write for as long as it lives, and a
    /// caller that waited on it could not be told to stop.
    writes: mpsc::Sender<Vec<u8>>,
    /// Why that thread stopped, when it did: the server closed its input or went away.
    write_failed: Arc<Mutex<Option<String>>>,
    lines: Receiver<Result<String, String>>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    next: u64,
    /// What the server said of how to use it, when it said (`initialize`'s `instructions`).
    instructions: Option<String>,
}

impl McpClient {
    /// Start `server` in `folder` and complete the handshake.
    pub fn start(
        server: &AuthorServer,
        folder: &Path,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<McpClient, McpError> {
        let mut child = Command::new(&server.command)
            .args(&server.args)
            .envs(server.env.iter().cloned())
            .current_dir(folder)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| McpError::Gone(format!("it could not be started: {e}")))?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        let (send, lines) = mpsc::channel();
        thread::spawn(move || read_lines(stdout, &send));
        let kept = Arc::new(Mutex::new(VecDeque::new()));
        let keep = Arc::clone(&kept);
        thread::spawn(move || keep_tail(stderr, &keep));
        let (writes, to_write) = mpsc::channel();
        let write_failed = Arc::new(Mutex::new(None));
        let failed = Arc::clone(&write_failed);
        thread::spawn(move || write_lines(stdin, &to_write, &failed));
        let mut client = McpClient {
            child,
            writes,
            write_failed,
            lines,
            stderr: kept,
            next: 1,
            instructions: None,
        };
        let said = client.request(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL,
                "capabilities": {},
                "clientInfo": {"name": "sce-workbench", "version": env!("CARGO_PKG_VERSION")},
            }),
            deadline,
            cancel,
        )?;
        client.instructions = said["instructions"]
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string);
        // Said as a notification, which wants no answer.
        client.write(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(client)
    }

    /// What the server says of how to use it, when it says anything.
    pub fn instructions(&self) -> Option<&str> {
        self.instructions.as_deref()
    }

    /// The tools the server has.
    pub fn tools(&mut self, deadline: Instant, cancel: &Cancel) -> Result<Vec<Tool>, McpError> {
        let said = self.request("tools/list", json!({}), deadline, cancel)?;
        let list = said["tools"]
            .as_array()
            .ok_or_else(|| McpError::Protocol("`tools/list` has no `tools` list".to_string()))?;
        list.iter()
            .map(|tool| {
                let name = tool["name"]
                    .as_str()
                    .ok_or_else(|| McpError::Protocol("a tool has no name".to_string()))?;
                Ok(Tool {
                    name: name.to_string(),
                    description: tool["description"].as_str().unwrap_or("").to_string(),
                    schema: tool
                        .get("inputSchema")
                        .cloned()
                        .unwrap_or_else(|| json!({"type": "object", "properties": {}})),
                })
            })
            .collect()
    }

    /// Call the tool `name` with `arguments` (an object).
    pub fn call(
        &mut self,
        name: &str,
        arguments: &Value,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<Called, McpError> {
        let said = self.request(
            "tools/call",
            json!({"name": name, "arguments": arguments}),
            deadline,
            cancel,
        )?;
        // What a tool gives is a list of parts, and the words are the text ones.
        let text = said["content"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        Ok(Called {
            text,
            is_error: said["isError"].as_bool().unwrap_or(false),
        })
    }

    /// Hand the server a message. It is written by the thread that owns the server's input, so this
    /// returns at once; that the server did not take it is found by the wait for its answer.
    fn write(&mut self, message: &Value) -> Result<(), McpError> {
        let mut line = message.to_string();
        line.push('\n');
        self.writes.send(line.into_bytes()).map_err(|_| {
            McpError::Gone(format!(
                "{}{}",
                self.write_failure()
                    .unwrap_or_else(|| "its input is closed".to_string()),
                self.said()
            ))
        })
    }

    /// Why the server's input could not be written, when it could not.
    fn write_failure(&self) -> Option<String> {
        self.write_failed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// What the server last said on its standard error, for a sentence about why it is gone.
    fn said(&self) -> String {
        let kept = self.stderr.lock().unwrap_or_else(|e| e.into_inner());
        let bytes: Vec<u8> = kept.iter().copied().collect();
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim();
        if text.is_empty() {
            String::new()
        } else {
            format!(" (it last said: {text})")
        }
    }

    /// One request, and the answer to it: what is said between is not it, and is passed over.
    fn request(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
        cancel: &Cancel,
    ) -> Result<Value, McpError> {
        let id = self.next;
        self.next += 1;
        self.write(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            if cancel.is_cancelled() {
                return Err(McpError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(McpError::TimedOut);
            }
            let line = match self.lines.recv_timeout(SLICE) {
                Ok(Ok(line)) => line,
                Ok(Err(why)) => return Err(McpError::Gone(format!("{why}{}", self.said()))),
                Err(RecvTimeoutError::Timeout) => {
                    // A server that closed its input and went on living is told by the write that
                    // failed, and not waited for until its time is up.
                    if let Some(why) = self.write_failure() {
                        return Err(McpError::Gone(format!("{why}{}", self.said())));
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(McpError::Gone(format!(
                        "it closed its output{}",
                        self.said()
                    )))
                }
            };
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                // Not a message: a server that prints a line of its own is not a reason to stop.
                continue;
            };
            if message["id"] != json!(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                let why = error["message"].as_str().unwrap_or("it gave no reason");
                return Err(McpError::Refused(why.to_string()));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // The server is not asked to finish: what it was doing is of no use to anybody now.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Write each message to the server, in order, until the messages end (the client was dropped) or
/// the server will not take one, and say why when it will not.
fn write_lines(
    mut stdin: ChildStdin,
    messages: &Receiver<Vec<u8>>,
    failed: &Mutex<Option<String>>,
) {
    for message in messages {
        if let Err(e) = stdin.write_all(&message).and_then(|()| stdin.flush()) {
            *failed.lock().unwrap_or_else(|e| e.into_inner()) = Some(e.to_string());
            return;
        }
    }
}

/// Send each line the server says, up to a most, and why it stopped.
fn read_lines(stdout: impl Read, send: &mpsc::Sender<Result<String, String>>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = Vec::new();
        // One byte over the most is how a line that is too long is told from one that is not.
        match reader
            .by_ref()
            .take(LINE_MAX as u64 + 1)
            .read_until(b'\n', &mut line)
        {
            Ok(0) => return,
            Ok(_) if line.len() > LINE_MAX => {
                let _ = send.send(Err("a line it said is longer than is read".to_string()));
                return;
            }
            Ok(_) => {
                let text = String::from_utf8_lossy(&line).trim().to_string();
                if !text.is_empty() && send.send(Ok(text)).is_err() {
                    return;
                }
            }
            Err(e) => {
                let _ = send.send(Err(e.to_string()));
                return;
            }
        }
    }
}

/// Keep the last of what the server says on its standard error.
fn keep_tail(stderr: impl Read, kept: &Mutex<VecDeque<u8>>) {
    let mut reader = BufReader::new(stderr);
    let mut chunk = [0u8; 1024];
    while let Ok(count) = reader.read(&mut chunk) {
        if count == 0 {
            return;
        }
        let mut kept = kept.lock().unwrap_or_else(|e| e.into_inner());
        kept.extend(&chunk[..count]);
        while kept.len() > STDERR_KEPT {
            kept.pop_front();
        }
    }
}
