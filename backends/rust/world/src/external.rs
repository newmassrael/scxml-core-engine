// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An actor whose machine runs in another process.
//!
//! The world runs generated Rust machines in its own process. A machine generated for
//! another language cannot be linked in, and a replay of what it did once is not a
//! run: what a world asks is whether copies still agree when the machines are different
//! implementations of one document, and for that the other implementation has to be one
//! of the copies. An [`ExternalMachine`] is that: a child process that holds one
//! generated machine and is driven by the world over its stdin and stdout.
//!
//! What stays in the world is everything a machine does not own: the virtual clock, the
//! seeded randomness, the network and the Mesh endpoint (`sce-rust-mesh`), which numbers,
//! retries, deduplicates and orders envelopes as it does for a Rust machine. What the
//! child owns is the machine. The seam between them is the one a Rust actor already has:
//! the events the endpoint hands the engine go to the child, and the `<send>`s the
//! machine makes to a peer come back and go to the endpoint as the runtime's
//! `HostSendRequest`. A language that follows needs a step server that speaks the
//! protocol below and nothing else from the world.
//!
//! # Determinism
//!
//! The child reads no wall time and draws no randomness of its own: its clock is a manual
//! one that only `advance` moves, and it does nothing between requests. Its replies are a
//! function of the requests it was given, so a run with an external actor is as
//! repeatable as one without.
//!
//! # The protocol
//!
//! Version 1. One request per line on the child's stdin, one reply per line on its
//! stdout, strictly in turn, each line a JSON object, UTF-8. Anything else the child
//! prints belongs on stderr, which the world keeps and shows when a run fails. The first
//! thing the world does is `hello`; a child that answers with another protocol version is
//! refused.
//!
//! | request | other members | reply |
//! |---|---|---|
//! | `hello` | | `ok`, `protocol` (a number), `machine` (the name it hosts) |
//! | `start` | `at_ms` | a step reply |
//! | `raise` | `event`, `data`, `send_id`, `origin`, `origin_type`, `invoke_id` (strings) | a step reply |
//! | `advance` | `ms` | a step reply |
//! | `saved` | | `ok`, `saved` (the machine's saved state, as the text it saves) |
//! | `restore` | `at_ms`, `saved` | a step reply |
//! | `shutdown` | | `ok`, then the child exits |
//!
//! `start` replaces whatever machine the child holds with a new one whose clock reads
//! `at_ms`, started and settled. `raise` gives the running machine an external event by
//! name, with the data as the JSON text the event carries (empty for none) and the
//! metadata a Mesh delivery gives it. `advance` moves the machine's clock on by `ms` and
//! runs what came due. `saved` reads the machine back. `restore` replaces the machine
//! with one stood where `saved` left a machine, with no entry action run and no data
//! evaluated, its clock reading `at_ms`. After every step the child settles the machine,
//! as the scenarios every backend replays do (docs/SCE_ACCEPTED_SUBSET.md §2.15), so the
//! world never has to know how many rounds a machine needs.
//!
//! A step reply is `{"ok":true,"sent":[...],"next_due_in_ms":N|null}`. `sent` is what the
//! machine asked to send its Mesh peers since the last reply, oldest first, each as
//! `{"processor_type","event","target","content","params","send_id","data","invoke_id"}`:
//! the fields the engine hands a host-served send, `data` being the event's data as the
//! engine serialised it, `params` an object from a name to the list of its values.
//! `next_due_in_ms` is how long from now the machine's scheduler has something to
//! deliver, `null` when nothing waits; a world reads it to know when the clock must reach
//! the machine again.
//!
//! A refusal is `{"ok":false,"error":"..."}`. Whatever the child answers that is not
//! this, a child that exits, and a child that does not answer in time are the same
//! thing to the world: a failed run, reported with the child's stderr
//! ([`ExternalFailure`]).
//!
//! # Not carried yet
//!
//! The end of a Mesh request (`<invoke type="sce:mesh-rpc">`) is not an event of the
//! machine and has no request here; an external actor whose endpoint produces one stops
//! the run. A recorded actor ([`World::add_read`](crate::World::add_read)) is a Rust
//! machine's.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{EngineCall, EngineEvent};
use sce_rust_runtime::{EventMetadata, HostSendRequest};
use serde_json::{json, Value};

use super::{ActorEndpoint, Clock, Participant, PUMP_LIMIT};

/// The version of the protocol this module speaks.
pub const PROTOCOL_VERSION: u64 = 1;

/// How long the child gets to answer `hello`: a JVM starting and loading a machine is
/// not a step.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);

/// How long the child gets to answer a step unless the command says otherwise.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

/// How long a child that was asked to shut down is waited for before it is killed.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

/// How much of the child's stderr is kept, from the end.
const STDERR_KEPT: usize = 64 * 1024;

/// What it takes to start the child: the program and what to pass it.
#[derive(Debug, Clone)]
pub struct ExternalCommand {
    program: PathBuf,
    args: Vec<OsString>,
    envs: Vec<(OsString, OsString)>,
    step_timeout: Duration,
}

impl ExternalCommand {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            envs: Vec::new(),
            step_timeout: STEP_TIMEOUT,
        }
    }

    #[must_use]
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.envs.push((key.into(), value.into()));
        self
    }

    /// How long the child gets to answer one step. A child that is slower is killed and
    /// the run fails, as one that hangs would otherwise hang the suite.
    #[must_use]
    pub fn step_timeout(mut self, timeout: Duration) -> Self {
        self.step_timeout = timeout;
        self
    }

    fn describe(&self) -> String {
        let mut text = self.program.display().to_string();
        for arg in &self.args {
            text.push(' ');
            text.push_str(&arg.to_string_lossy());
        }
        text
    }
}

/// Why a child could not be driven: what went wrong, and what the child had written to
/// its stderr by then, which is where it says why.
#[derive(Debug, Clone)]
pub struct ExternalFailure {
    pub what: String,
    pub stderr: String,
}

impl fmt::Display for ExternalFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.what)?;
        if self.stderr.trim().is_empty() {
            write!(f, "\nthe child wrote nothing to stderr")
        } else {
            write!(f, "\nthe child's stderr:\n{}", self.stderr.trim_end())
        }
    }
}

impl std::error::Error for ExternalFailure {}

/// What a step left: what the machine sent its peers, and when it next has something due.
#[derive(Debug, Clone, Default)]
pub(crate) struct Stepped {
    pub sent: Vec<HostSendRequest>,
    pub next_due_in_ms: Option<u64>,
}

/// The child process and the lines that go to and from it.
struct Process {
    command: String,
    child: Child,
    stdin: Option<ChildStdin>,
    replies: Receiver<std::io::Result<String>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    readers: Vec<JoinHandle<()>>,
    step_timeout: Duration,
    /// The name of the machine the child hosts, from `hello`.
    machine: String,
    /// Whether a call has failed: nothing is asked of a child that has, since what it
    /// would answer is no longer what the request meant.
    failed: bool,
}

impl Process {
    fn spawn(command: &ExternalCommand) -> Result<Self, ExternalFailure> {
        let described = command.describe();
        let mut spawned = Command::new(&command.program);
        spawned
            .args(&command.args)
            .envs(command.envs.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = spawned.spawn().map_err(|error| ExternalFailure {
            what: format!("could not start `{described}`: {error}"),
            stderr: String::new(),
        })?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr_pipe = child.stderr.take().expect("stderr was piped");

        let (lines, replies) = mpsc::channel();
        let stdout_reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let failed = line.is_err();
                if lines.send(line).is_err() || failed {
                    return;
                }
            }
        });
        let stderr: Arc<Mutex<Vec<u8>>> = Arc::default();
        let kept = Arc::clone(&stderr);
        let stderr_reader = std::thread::spawn(move || {
            let mut pipe = stderr_pipe;
            let mut chunk = [0u8; 4096];
            while let Ok(read) = pipe.read(&mut chunk) {
                if read == 0 {
                    return;
                }
                let mut kept = kept
                    .lock()
                    .expect("the stderr buffer is never left mid-update");
                kept.extend_from_slice(&chunk[..read]);
                let excess = kept.len().saturating_sub(STDERR_KEPT);
                kept.drain(..excess);
            }
        });

        let mut process = Self {
            command: described,
            child,
            stdin,
            replies,
            stderr,
            readers: vec![stdout_reader, stderr_reader],
            step_timeout: command.step_timeout,
            machine: String::new(),
            failed: false,
        };
        let hello = process.call(&json!({ "op": "hello" }), STARTUP_TIMEOUT)?;
        let protocol = hello.get("protocol").and_then(Value::as_u64);
        if protocol != Some(PROTOCOL_VERSION) {
            return Err(process.fail(format!(
                "`{}` speaks protocol {protocol:?}, and this world speaks {PROTOCOL_VERSION}",
                process.command
            )));
        }
        process.machine = process.string(&hello, "machine")?;
        Ok(process)
    }

    /// Record that the run cannot go on, and give the failure, with what the child said.
    /// The child is killed: nothing it could still do is wanted.
    fn fail(&mut self, what: String) -> ExternalFailure {
        self.failed = true;
        let status = match self.child.try_wait() {
            Ok(Some(status)) => format!(" (the child had exited: {status})"),
            _ => String::new(),
        };
        // `kill` fails for a child that has exited already, which is the answer wanted.
        let _ = self.child.kill();
        let _ = self.child.wait();
        // The readers end once the pipes close, so what the child wrote last is in the
        // buffer by the time they have been joined.
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
        let stderr = String::from_utf8_lossy(
            &self
                .stderr
                .lock()
                .expect("the stderr buffer is never left mid-update"),
        )
        .into_owned();
        ExternalFailure {
            what: format!("{what}{status}"),
            stderr,
        }
    }

    /// One request and its reply, which is refused when it says so.
    fn call(&mut self, request: &Value, timeout: Duration) -> Result<Value, ExternalFailure> {
        if self.failed {
            return Err(self.fail(format!(
                "`{}` failed earlier and is asked nothing more",
                self.command
            )));
        }
        let op = request["op"].as_str().unwrap_or("?").to_string();
        let mut line = request.to_string();
        line.push('\n');
        let written = self
            .stdin
            .as_mut()
            .ok_or_else(|| "its stdin is closed".to_string())
            .and_then(|stdin| {
                stdin
                    .write_all(line.as_bytes())
                    .and_then(|()| stdin.flush())
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = written {
            return Err(self.fail(format!(
                "could not send `{op}` to `{}`: {error}",
                self.command
            )));
        }
        let reply = match self.replies.recv_timeout(timeout) {
            Ok(Ok(reply)) => reply,
            Ok(Err(error)) => {
                return Err(self.fail(format!(
                    "could not read the reply to `{op}` from `{}`: {error}",
                    self.command
                )))
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(self.fail(format!(
                    "`{}` did not answer `{op}` within {timeout:?}",
                    self.command
                )))
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(self.fail(format!(
                    "`{}` closed its stdout before it answered `{op}`",
                    self.command
                )))
            }
        };
        let value: Value = match serde_json::from_str(&reply) {
            Ok(value) => value,
            Err(error) => {
                return Err(self.fail(format!(
                    "`{}` answered `{op}` with a line that is not JSON ({error}): {reply}",
                    self.command
                )))
            }
        };
        match value.get("ok").and_then(Value::as_bool) {
            Some(true) => Ok(value),
            Some(false) => {
                let error = value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("(no reason given)")
                    .to_string();
                Err(self.fail(format!("`{}` refused `{op}`: {error}", self.command)))
            }
            None => Err(self.fail(format!(
                "`{}` answered `{op}` with a line that has no `ok`: {reply}",
                self.command
            ))),
        }
    }

    fn step(&mut self, request: &Value) -> Result<Stepped, ExternalFailure> {
        let timeout = self.step_timeout;
        let reply = self.call(request, timeout)?;
        let mut sent = Vec::new();
        let listed = reply
            .get("sent")
            .and_then(Value::as_array)
            .ok_or_else(|| self.fail(format!("a step reply has no `sent` list: {reply}")))?;
        for one in listed {
            sent.push(self.send_request(one)?);
        }
        let next_due_in_ms = match reply.get("next_due_in_ms") {
            Some(Value::Null) => None,
            Some(due) => Some(due.as_u64().ok_or_else(|| {
                self.fail(format!("`next_due_in_ms` is not a whole number: {due}"))
            })?),
            None => return Err(self.fail(format!("a step reply has no `next_due_in_ms`: {reply}"))),
        };
        Ok(Stepped {
            sent,
            next_due_in_ms,
        })
    }

    fn string(&mut self, object: &Value, name: &str) -> Result<String, ExternalFailure> {
        match object.get(name).and_then(Value::as_str) {
            Some(text) => Ok(text.to_string()),
            None => Err(self.fail(format!("`{name}` is missing or not a string in {object}"))),
        }
    }

    fn send_request(&mut self, one: &Value) -> Result<HostSendRequest, ExternalFailure> {
        let mut params: HashMap<String, Vec<String>> = HashMap::new();
        let listed = one
            .get("params")
            .and_then(Value::as_object)
            .ok_or_else(|| self.fail(format!("a send has no `params` object: {one}")))?;
        for (name, values) in listed {
            let values = values
                .as_array()
                .and_then(|values| {
                    values
                        .iter()
                        .map(|value| value.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or_else(|| {
                    self.fail(format!(
                        "param `{name}` of a send is not a list of strings: {one}"
                    ))
                })?;
            params.insert(name.clone(), values);
        }
        Ok(HostSendRequest {
            processor_type: self.string(one, "processor_type")?,
            event_name: self.string(one, "event")?,
            target: self.string(one, "target")?,
            content: self.string(one, "content")?,
            params,
            send_id: self.string(one, "send_id")?,
            event_data: self.string(one, "data")?,
            invoke_id: self.string(one, "invoke_id")?,
        })
    }
}

impl Drop for Process {
    /// Ask the child to go, give it a moment, and kill what is left, so that no test
    /// leaves a process behind, whether it passed or panicked.
    fn drop(&mut self) {
        if !self.failed {
            if let Some(stdin) = self.stdin.as_mut() {
                let _ = stdin.write_all(b"{\"op\":\"shutdown\"}\n");
                let _ = stdin.flush();
            }
            // The end of stdin is a shutdown too, for a child that missed the request.
            self.stdin = None;
            let until = Instant::now() + SHUTDOWN_GRACE;
            while Instant::now() < until {
                match self.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                    Err(_) => break,
                }
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

/// A handle on a child process that hosts one machine, which any number of worlds may be
/// given in turn.
///
/// Starting a child costs far more than a step (a JVM loading a machine takes seconds),
/// so a test spawns one per actor and gives a clone of the handle to each world it
/// builds: [`World::add_external`](crate::World::add_external) starts the machine afresh
/// in the same child. Two worlds that run at once must not share a handle, as the second
/// would start over the machine the first is using. The child is shut down when the last
/// clone is dropped.
#[derive(Clone)]
pub struct ExternalMachine {
    process: Arc<Mutex<Process>>,
}

impl fmt::Debug for ExternalMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalMachine")
            .field("machine", &self.machine())
            .finish()
    }
}

impl ExternalMachine {
    /// Start the child and greet it.
    ///
    /// # Errors
    ///
    /// When the program cannot be started, does not answer, or speaks another version of
    /// the protocol.
    pub fn spawn(command: &ExternalCommand) -> Result<Self, ExternalFailure> {
        Ok(Self {
            process: Arc::new(Mutex::new(Process::spawn(command)?)),
        })
    }

    /// The name of the machine the child hosts, as it said at `hello`.
    pub fn machine(&self) -> String {
        self.lock().machine.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Process> {
        self.process
            .lock()
            .expect("the child's pipes are never left mid-update")
    }

    pub(crate) fn start(&self, at_ms: u64) -> Result<Stepped, ExternalFailure> {
        self.lock().step(&json!({ "op": "start", "at_ms": at_ms }))
    }

    pub(crate) fn raise(&self, event: &EngineEvent) -> Result<Stepped, ExternalFailure> {
        let meta = &event.metadata;
        self.lock().step(&json!({
            "op": "raise",
            "event": event.name,
            "data": meta.data,
            "send_id": meta.send_id,
            "origin": meta.origin,
            "origin_type": meta.origin_type,
            "invoke_id": meta.invoke_id,
        }))
    }

    pub(crate) fn advance(&self, ms: u64) -> Result<Stepped, ExternalFailure> {
        self.lock().step(&json!({ "op": "advance", "ms": ms }))
    }

    pub(crate) fn saved(&self) -> Result<String, ExternalFailure> {
        let mut process = self.lock();
        let timeout = process.step_timeout;
        let reply = process.call(&json!({ "op": "saved" }), timeout)?;
        process.string(&reply, "saved")
    }

    pub(crate) fn restore(&self, at_ms: u64, saved: &str) -> Result<Stepped, ExternalFailure> {
        self.lock()
            .step(&json!({ "op": "restore", "at_ms": at_ms, "saved": saved }))
    }
}

/// An actor of a world whose machine is a child process.
pub(crate) struct ExternalActor {
    name: String,
    machine: ExternalMachine,
    endpoint: ActorEndpoint,
    clock: Clock,
    /// What the host gave the machine that it has not taken yet, oldest first.
    given: Vec<(String, String)>,
    /// How far the world's time has moved that the machine has not been told. A machine
    /// is told when something could come of it: before it is given an event or read, and
    /// when its scheduler has something due. Telling it at every move would be a round
    /// trip for each one, and a machine with nothing due cannot tell the difference.
    owed_ms: u64,
    /// When the machine's scheduler next has something to deliver, in the world's
    /// milliseconds, as of the last time it answered.
    due_ms: Option<i64>,
}

impl ExternalActor {
    /// Start the machine at the world's present time.
    pub(crate) fn begin(
        name: &str,
        machine: &ExternalMachine,
        endpoint: ActorEndpoint,
        clock: Clock,
    ) -> Self {
        let mut actor = Self {
            name: name.to_string(),
            machine: machine.clone(),
            endpoint,
            clock,
            given: Vec::new(),
            owed_ms: 0,
            due_ms: None,
        };
        let stepped = actor.demand(actor.machine.start(actor.now()));
        actor.take(stepped);
        actor.pump();
        actor
    }

    fn now(&self) -> u64 {
        u64::try_from(self.clock.now_ms()).unwrap_or(0)
    }

    /// The answer of a request, or the failure of the run.
    fn demand<T>(&self, answer: Result<T, ExternalFailure>) -> T {
        match answer {
            Ok(answer) => answer,
            Err(failure) => panic!("{}: {failure}", self.name),
        }
    }

    /// Take in what a step left: what the machine sent goes to the endpoint, in order,
    /// and the machine's next deadline is noted.
    fn take(&mut self, stepped: Stepped) {
        {
            let mut endpoint = self.lock_endpoint();
            for request in &stepped.sent {
                endpoint.send(request);
            }
        }
        self.due_ms = stepped.next_due_in_ms.map(|wait| {
            self.clock
                .now_ms()
                .saturating_add(i64::try_from(wait).unwrap_or(i64::MAX))
        });
    }

    /// Tell the machine the time that moved since it was last told.
    fn catch_up(&mut self) {
        let owed = std::mem::take(&mut self.owed_ms);
        if owed > 0 {
            let stepped = self.demand(self.machine.advance(owed));
            self.take(stepped);
        }
    }

    /// Give the machine one event, once it has been told the time.
    fn deliver(&mut self, event: &EngineEvent) {
        self.catch_up();
        let stepped = self.demand(self.machine.raise(event));
        self.take(stepped);
    }

    /// Stop the machine and start it again from `saved`, or from what it saved of itself
    /// when none is given; what it had been given and not yet taken is lost.
    pub(crate) fn restart(&mut self, saved: Option<&str>) {
        self.catch_up();
        let saved = match saved {
            Some(saved) => saved.to_string(),
            None => self.demand(self.machine.saved()),
        };
        let stepped = self.demand(self.machine.restore(self.now(), &saved));
        self.take(stepped);
        self.given.clear();
    }

    fn pump(&mut self) {
        for _ in 0..PUMP_LIMIT {
            // A machine with nothing due is left with the time it is owed; one whose
            // deadline the world's time has reached is told, so that it delivers.
            if self.due_ms.is_some_and(|due| due <= self.clock.now_ms()) {
                self.catch_up();
            }
            let given = std::mem::take(&mut self.given);
            let calls = self.lock_endpoint().take_calls();
            if given.is_empty() && calls.is_empty() {
                return;
            }
            for (name, data) in given {
                let metadata = EventMetadata {
                    data,
                    ..EventMetadata::default()
                };
                self.deliver(&EngineEvent { name, metadata });
            }
            for call in calls {
                match call {
                    EngineCall::Raise(event) => self.deliver(&event),
                    EngineCall::EndInvoke { .. } => panic!(
                        "{} is hosted by another process, and the end of a Mesh request is not \
                         a request of the step protocol",
                        self.name
                    ),
                }
            }
        }
        panic!(
            "{} did not come to rest after {PUMP_LIMIT} rounds: it answers itself without end",
            self.name
        );
    }
}

impl Participant for ExternalActor {
    fn name(&self) -> &str {
        &self.name
    }

    fn endpoint(&self) -> &ActorEndpoint {
        &self.endpoint
    }

    fn raise(&mut self, event: &str, data: &str) {
        self.given.push((event.to_string(), data.to_string()));
    }

    fn pump(&mut self) {
        ExternalActor::pump(self);
    }

    fn advance(&mut self, ms: u64) {
        self.owed_ms = self.owed_ms.saturating_add(ms);
    }

    fn engine_due_ms(&self, _now_ms: i64) -> Option<i64> {
        self.due_ms
    }

    fn saved_state(&mut self) -> Option<String> {
        self.catch_up();
        Some(self.demand(self.machine.saved()))
    }

    fn restart_external(&mut self, saved: Option<&str>) -> bool {
        self.restart(saved);
        true
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_program_is_a_failure_that_names_it() {
        let failure = ExternalMachine::spawn(&ExternalCommand::new("/nonexistent/step-server"))
            .expect_err("there is no such program");
        assert!(
            failure.what.contains("/nonexistent/step-server"),
            "{failure}"
        );
    }

    #[test]
    fn a_child_that_dies_is_reported_with_what_it_wrote_to_stderr() {
        let failure = ExternalMachine::spawn(
            &ExternalCommand::new("sh").args(["-c", "echo it broke >&2; exit 3"]),
        )
        .expect_err("the child never greets");
        assert!(failure.stderr.contains("it broke"), "{failure}");
        assert!(failure.what.contains("exit"), "{failure}");
    }

    #[test]
    fn a_child_that_does_not_answer_in_time_fails_the_run_and_is_killed() {
        // The hello is answered and then the child goes quiet: what is held is that a
        // child that never answers a step cannot hang the suite.
        let quiet = ExternalCommand::new("sh")
            .args([
                "-c",
                r#"read line; echo '{"ok":true,"protocol":1,"machine":"quiet"}'; sleep 30"#,
            ])
            .step_timeout(Duration::from_millis(200));
        let machine = ExternalMachine::spawn(&quiet).expect("it greets");
        assert_eq!(machine.machine(), "quiet");
        let failure = machine.advance(1).expect_err("it never answers a step");
        assert!(failure.what.contains("did not answer"), "{failure}");
    }

    #[test]
    fn a_child_that_speaks_another_protocol_is_refused() {
        let failure = ExternalMachine::spawn(&ExternalCommand::new("sh").args([
            "-c",
            r#"read line; echo '{"ok":true,"protocol":99,"machine":"x"}'"#,
        ]))
        .expect_err("protocol 99 is not 1");
        assert!(failure.what.contains("protocol"), "{failure}");
    }

    #[test]
    fn a_line_that_is_not_json_is_a_failure_that_shows_the_line() {
        let failure = ExternalMachine::spawn(
            &ExternalCommand::new("sh")
                .args(["-c", "read line; echo 'Picked up JAVA_TOOL_OPTIONS'"]),
        )
        .expect_err("that is not a reply");
        assert!(failure.what.contains("Picked up"), "{failure}");
    }

    #[test]
    fn a_refusal_carries_the_childs_reason() {
        let machine = ExternalMachine::spawn(&ExternalCommand::new("sh").args([
            "-c",
            r#"read line; echo '{"ok":true,"protocol":1,"machine":"x"}'; read line; echo '{"ok":false,"error":"no machine is running"}'"#,
        ]))
        .expect("it greets");
        let failure = machine.advance(1).expect_err("it refuses");
        assert!(failure.what.contains("no machine is running"), "{failure}");
    }

    #[test]
    fn what_a_step_reports_is_read_back_as_the_send_the_engine_made() {
        let machine = ExternalMachine::spawn(&ExternalCommand::new("sh").args([
            "-c",
            concat!(
                r#"read line; echo '{"ok":true,"protocol":1,"machine":"x"}'; read line; "#,
                r##"echo '{"ok":true,"sent":[{"processor_type":"sce:mesh","event":"todo.write","target":"#server","content":"","params":{"id":["1"]},"send_id":"s1","data":"{\"id\":1}","invoke_id":""}],"next_due_in_ms":40}'"##
            ),
        ]))
        .expect("it greets");
        let stepped = machine.advance(1).expect("a step reply");
        assert_eq!(stepped.next_due_in_ms, Some(40));
        let [send] = stepped.sent.as_slice() else {
            panic!("one send was reported: {stepped:?}")
        };
        assert_eq!(send.target, "#server");
        assert_eq!(send.event_name, "todo.write");
        assert_eq!(send.event_data, r#"{"id":1}"#);
        assert_eq!(send.params["id"], ["1"]);
        assert_eq!(send.send_id, "s1");
    }

    #[test]
    fn a_dropped_handle_ends_a_child_that_reads_until_its_stdin_closes() {
        // `cat` ends at the end of its stdin and nothing else ends it: were the pipe left
        // open, the drop would wait out the grace and kill it instead.
        let machine = ExternalMachine::spawn(&ExternalCommand::new("sh").args([
            "-c",
            r#"read line; echo '{"ok":true,"protocol":1,"machine":"x"}'; cat > /dev/null"#,
        ]))
        .expect("it greets");
        let started = Instant::now();
        drop(machine);
        assert!(
            started.elapsed() < SHUTDOWN_GRACE,
            "the child was killed at the end of the grace, not asked to go"
        );
    }

    #[test]
    fn a_dropped_handle_kills_a_child_that_ignores_the_request() {
        let machine = ExternalMachine::spawn(&ExternalCommand::new("sh").args([
            "-c",
            r#"read line; echo '{"ok":true,"protocol":1,"machine":"x"}'; exec sleep 60"#,
        ]))
        .expect("it greets");
        let started = Instant::now();
        drop(machine);
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "a child that ignores the request is killed, not waited for"
        );
    }
}
