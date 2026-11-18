// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The parent's handle on a server process: start it, feed it commands, read its reports, and
//! end it, by `SIGKILL` or by closing its input.
//!
//! Commands are written by a thread of their own and reports read by another, so that the parent
//! never blocks on a pipe the server stopped reading (a server halted at a point the parent chose
//! reads nothing until it is killed) and a report is timed when it is read, not when the parent
//! gets to it.

use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command as Process, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::clock::wall_ms;
use crate::protocol::{Command, Report, ServerOptions};

/// A report and the wall-clock moment the parent read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub at_ms: u64,
    pub report: Report,
}

/// A running server process. Dropped without being ended, it is killed, so a scenario that panics
/// leaves no server behind.
pub struct ServerProcess {
    child: Child,
    input: Option<Sender<String>>,
    writer: Option<JoinHandle<()>>,
    reports: Receiver<Reported>,
    reader: Option<JoinHandle<()>>,
    ended: bool,
}

impl ServerProcess {
    /// Start `program` (the suite's `sce-suite-server`) with `options`. Its standard error is the
    /// test's, so what it says when it fails to start is seen.
    pub fn start(program: &Path, options: &ServerOptions) -> io::Result<ServerProcess> {
        let mut child = Process::new(program)
            .args(options.args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("the server's streams were not piped"));
        };
        let (input, lines) = mpsc::channel::<String>();
        let writer = thread::spawn(move || {
            let mut stdin = stdin;
            for line in lines {
                if writeln!(stdin, "{line}")
                    .and_then(|()| stdin.flush())
                    .is_err()
                {
                    // The server is gone; what is left to write reaches no one.
                    return;
                }
            }
        });
        let (sink, reports) = mpsc::channel();
        let reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    return;
                };
                let at_ms = wall_ms();
                let report = Report::parse(&line)
                    .unwrap_or_else(|| Report::Error(format!("a line that is no report: {line}")));
                if sink.send(Reported { at_ms, report }).is_err() {
                    return;
                }
            }
        });
        Ok(ServerProcess {
            child,
            input: Some(input),
            writer: Some(writer),
            reports,
            reader: Some(reader),
            ended: false,
        })
    }

    /// The process's id.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Hand the server `command`. Never blocks: the command waits for the writer.
    pub fn send(&self, command: &Command) {
        if let Some(input) = &self.input {
            // A writer that ended had a server that ended; the command reaches no one.
            let _ = input.send(command.line());
        }
    }

    /// The next report, waiting at most `wait` for it.
    pub fn report_within(&self, wait: Duration) -> Option<Reported> {
        self.reports.recv_timeout(wait).ok()
    }

    /// Wait at most `patience` for the server to say it serves; the reports it made before, or
    /// why it never said so.
    pub fn await_ready(&self, patience: Duration) -> Result<Vec<Reported>, String> {
        let deadline = Instant::now() + patience;
        let mut before = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(format!(
                    "the server did not say it serves within {patience:?}; it said {before:?}"
                ));
            }
            match self.reports.recv_timeout(left) {
                Ok(Reported {
                    report: Report::Ready,
                    ..
                }) => return Ok(before),
                Ok(other) => before.push(other),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "the server ended before it said it serves; it said {before:?}"
                    ))
                }
            }
        }
    }

    /// Kill the server with `SIGKILL` (on Unix; the process is terminated at once elsewhere) and
    /// answer what it reported that was not read yet: everything it wrote before it died.
    pub fn kill(mut self) -> Vec<Reported> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.finish()
    }

    /// Close the server's input, which ends it once it has taken every command before; kill it if
    /// it has not ended within `patience`. Answers what it reported that was not read yet.
    pub fn stop(mut self, patience: Duration) -> Vec<Reported> {
        self.input = None;
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        let deadline = Instant::now() + patience;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
            }
        }
        self.finish()
    }

    /// The process has ended: join its threads and answer what it reported that was not read.
    fn finish(&mut self) -> Vec<Reported> {
        self.ended = true;
        self.input = None;
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.reports.try_iter().collect()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if !self.ended {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
