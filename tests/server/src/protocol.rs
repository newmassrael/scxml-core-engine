// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the suite's server process reads and writes, a line at a time.
//!
//! The server (`sce-suite-server`) serves the [`ledger`](crate::ledger) machine as a keyed pool on
//! the reference store, and carries the envelopes of its one client over its standard streams:
//! the parent writes a [`Command`] to its standard input for each envelope the client sends, and
//! reads a [`Report`] from its standard output for each envelope the pool transmits and each
//! write the store took or refused. A line is written whole with one `write`, so what the parent
//! read before a kill is exactly what the server sent before it: the client saw it.
//!
//! [`ServerOptions`] is the command line, with the [`Halt`] a server stops at so that it is killed
//! at a point of load, checkpoint or delivery the parent chose.

use std::fmt;
use std::path::PathBuf;

use crate::wire::{hex, unhex};

/// Where a server may stop and wait to be killed: the moment of its work the parent chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// A session's checkpoint is read from the store: a session is being loaded, its lease taken.
    Load,
    /// A checkpoint is about to be written: the input that made it ran, nothing of it is durable.
    CheckpointBefore,
    /// A checkpoint was written and is durable, and the pool has not yet released what it holds.
    CheckpointAfter,
    /// An envelope is about to be handed to the client.
    SendBefore,
    /// An envelope was handed to the client.
    SendAfter,
}

impl Stage {
    /// Every stage, in the order of a session's work.
    pub const ALL: [Stage; 5] = [
        Stage::Load,
        Stage::CheckpointBefore,
        Stage::CheckpointAfter,
        Stage::SendBefore,
        Stage::SendAfter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Load => "load",
            Stage::CheckpointBefore => "checkpoint-before",
            Stage::CheckpointAfter => "checkpoint-after",
            Stage::SendBefore => "send-before",
            Stage::SendAfter => "send-after",
        }
    }

    pub fn from_name(name: &str) -> Option<Stage> {
        Stage::ALL.into_iter().find(|stage| stage.name() == name)
    }
}

/// The `at`th time a server reaches `stage` in its life, counted from 1, where it stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Halt {
    pub stage: Stage,
    pub at: u64,
}

impl Halt {
    /// The halt `text` writes as [`fmt::Display`] writes one: `<stage>:<count>`.
    pub fn parse(text: &str) -> Option<Halt> {
        let (stage, at) = text.split_once(':')?;
        Some(Halt {
            stage: Stage::from_name(stage)?,
            at: at.parse().ok().filter(|at| *at > 0)?,
        })
    }
}

impl fmt::Display for Halt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.stage.name(), self.at)
    }
}

/// Why the store refused a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The write's fence is not the current lease's: another process took the key over.
    Fence,
    /// The session's version moved since the writer read it.
    Version,
}

impl Refusal {
    fn name(self) -> &'static str {
        match self {
            Refusal::Fence => "fence",
            Refusal::Version => "version",
        }
    }
}

/// A line of the server's standard output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The store is open and the pool serves: commands are read from here on.
    Ready,
    /// The pool handed the client this envelope.
    Sent(Vec<u8>),
    /// The store took a checkpoint of `key` under the lease of fence `fence`, at `version`.
    Wrote {
        key: String,
        version: u64,
        fence: u64,
    },
    /// The store refused a checkpoint of `key` made under the fence `fence`.
    Refused {
        key: String,
        fence: u64,
        refusal: Refusal,
    },
    /// The server reached its halt and waits to be killed.
    Halted(Halt),
    /// Something the pool told its host, in its debug form.
    Error(String),
}

impl Report {
    /// The line, without its end.
    pub fn line(&self) -> String {
        match self {
            Report::Ready => "ready".to_string(),
            Report::Sent(bytes) => format!("send {}", hex(bytes)),
            Report::Wrote {
                key,
                version,
                fence,
            } => format!("wrote {key} {version} {fence}"),
            Report::Refused {
                key,
                fence,
                refusal,
            } => format!("refused {key} {fence} {}", refusal.name()),
            Report::Halted(halt) => format!("halt {halt}"),
            Report::Error(text) => format!("error {}", text.replace(['\n', '\r'], " ")),
        }
    }

    /// The report `line` is, or `None` for a line that is none.
    pub fn parse(line: &str) -> Option<Report> {
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        let mut fields = rest.split(' ');
        match word {
            "ready" => Some(Report::Ready),
            "send" => unhex(rest).map(Report::Sent),
            "wrote" => Some(Report::Wrote {
                key: fields.next()?.to_string(),
                version: fields.next()?.parse().ok()?,
                fence: fields.next()?.parse().ok()?,
            }),
            "refused" => Some(Report::Refused {
                key: fields.next()?.to_string(),
                fence: fields.next()?.parse().ok()?,
                refusal: match fields.next()? {
                    "fence" => Refusal::Fence,
                    "version" => Refusal::Version,
                    _ => return None,
                },
            }),
            "halt" => Halt::parse(rest).map(Report::Halted),
            "error" => Some(Report::Error(rest.to_string())),
            _ => None,
        }
    }
}

/// A line of the server's standard input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// The client sent this envelope.
    Envelope(Vec<u8>),
    /// Stop serving for this many milliseconds, renewing no lease, as a process stalled by its
    /// machine does; then take the next command before anything else.
    Pause(u64),
}

impl Command {
    /// The line, without its end.
    pub fn line(&self) -> String {
        match self {
            Command::Envelope(bytes) => format!("envelope {}", hex(bytes)),
            Command::Pause(ms) => format!("pause {ms}"),
        }
    }

    /// The command `line` is, or `None` for a line that is none.
    pub fn parse(line: &str) -> Option<Command> {
        let (word, rest) = line.split_once(' ')?;
        match word {
            "envelope" => unhex(rest).map(Command::Envelope),
            "pause" => rest.parse().ok().map(Command::Pause),
            _ => None,
        }
    }
}

/// How a server is started: its command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerOptions {
    /// The store's database file.
    pub store: PathBuf,
    /// The name it holds leases under. A server started again after a kill takes the same one,
    /// as a process that restarts under its name does, and its new leases fence out its old life.
    pub owner: String,
    /// deploy.yaml's `sessions.idle_ms`: how long a quiet session stays in memory.
    pub idle_ms: u64,
    /// How long a lease lasts unless it is renewed.
    pub lease_ttl_ms: u64,
    /// How long after its grant or renewal a lease is renewed.
    pub renew_every_ms: u64,
    /// Where it stops and waits to be killed, if anywhere.
    pub halt: Option<Halt>,
}

impl ServerOptions {
    /// The command line that starts a server with these options.
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "--store".to_string(),
            self.store.to_string_lossy().into_owned(),
            "--owner".to_string(),
            self.owner.clone(),
            "--idle-ms".to_string(),
            self.idle_ms.to_string(),
            "--lease-ttl-ms".to_string(),
            self.lease_ttl_ms.to_string(),
            "--renew-every-ms".to_string(),
            self.renew_every_ms.to_string(),
        ];
        if let Some(halt) = self.halt {
            args.push("--halt".to_string());
            args.push(halt.to_string());
        }
        args
    }

    /// The options `args` (the command line after the program's name) give, or why they give
    /// none. Every option takes a value; every one but `--halt` must be given.
    pub fn from_args(args: &[String]) -> Result<ServerOptions, String> {
        let number = |flag: &str, value: &str| {
            value
                .parse::<u64>()
                .map_err(|_| format!("{flag} takes a count of milliseconds, not '{value}'"))
        };
        let (mut store, mut owner, mut halt) = (None, None, None);
        let (mut idle_ms, mut lease_ttl_ms, mut renew_every_ms) = (None, None, None);
        for pair in args.chunks(2) {
            let [flag, value] = pair else {
                return Err(format!("{} takes a value", pair[0]));
            };
            match flag.as_str() {
                "--store" => store = Some(PathBuf::from(value)),
                "--owner" => owner = Some(value.clone()),
                "--idle-ms" => idle_ms = Some(number(flag, value)?),
                "--lease-ttl-ms" => lease_ttl_ms = Some(number(flag, value)?),
                "--renew-every-ms" => renew_every_ms = Some(number(flag, value)?),
                "--halt" => {
                    halt = Some(
                        Halt::parse(value)
                            .ok_or_else(|| format!("--halt {value} names no stage and count"))?,
                    )
                }
                other => return Err(format!("{other} is no option of the suite's server")),
            }
        }
        let required = |name: &str| format!("{name} is required");
        Ok(ServerOptions {
            store: store.ok_or_else(|| required("--store"))?,
            owner: owner.ok_or_else(|| required("--owner"))?,
            idle_ms: idle_ms.ok_or_else(|| required("--idle-ms"))?,
            lease_ttl_ms: lease_ttl_ms.ok_or_else(|| required("--lease-ttl-ms"))?,
            renew_every_ms: renew_every_ms.ok_or_else(|| required("--renew-every-ms"))?,
            halt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_report_is_read_back_as_the_report_it_writes() {
        let reports = [
            Report::Ready,
            Report::Sent(vec![0xa1, 0x00, 0x7f]),
            Report::Wrote {
                key: "s12".to_string(),
                version: 40,
                fence: 7,
            },
            Report::Refused {
                key: "v0".to_string(),
                fence: 3,
                refusal: Refusal::Fence,
            },
            Report::Refused {
                key: "v0".to_string(),
                fence: 3,
                refusal: Refusal::Version,
            },
            Report::Halted(Halt {
                stage: Stage::CheckpointAfter,
                at: 12,
            }),
            Report::Error("LeaseHeld { key: \"k\" }".to_string()),
        ];
        for report in reports {
            assert_eq!(
                Report::parse(&report.line()),
                Some(report.clone()),
                "{report:?}"
            );
        }
        assert_eq!(
            Report::Error("two\nlines".to_string()).line(),
            "error two lines",
            "a report is one line, whatever its text holds"
        );
        assert_eq!(Report::parse("sent 00"), None);
        assert_eq!(Report::parse("halt load:0"), None, "a halt counts from 1");
    }

    #[test]
    fn every_command_is_read_back_as_the_command_it_writes() {
        for command in [Command::Envelope(vec![1, 2, 255]), Command::Pause(2500)] {
            assert_eq!(Command::parse(&command.line()), Some(command.clone()));
        }
        assert_eq!(Command::parse("envelope 0"), None);
        assert_eq!(Command::parse("pause soon"), None);
    }

    #[test]
    fn options_are_read_back_from_the_command_line_they_write() {
        let options = ServerOptions {
            store: PathBuf::from("/tmp/x/sessions.db"),
            owner: "a".to_string(),
            idle_ms: 1_000,
            lease_ttl_ms: 30_000,
            renew_every_ms: 10_000,
            halt: Some(Halt {
                stage: Stage::SendBefore,
                at: 3,
            }),
        };
        assert_eq!(
            ServerOptions::from_args(&options.args()),
            Ok(options.clone())
        );
        let without = ServerOptions {
            halt: None,
            ..options
        };
        assert_eq!(
            ServerOptions::from_args(&without.args()),
            Ok(without.clone())
        );
        let mut odd = without.args();
        odd.push("--halt".to_string());
        assert!(
            ServerOptions::from_args(&odd).is_err(),
            "a flag with no value"
        );
        assert!(
            ServerOptions::from_args(&without.args()[2..]).is_err(),
            "no store"
        );
        assert!(ServerOptions::from_args(&["--port".to_string(), "1".to_string()]).is_err());
    }
}
