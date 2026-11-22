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
    /// An acknowledgement is about to delete what it covers of a durable client's outbox
    /// (docs/adr/0109, D10).
    AckBefore,
    /// An acknowledgement deleted what it covered, durably, and the server has not answered yet.
    AckAfter,
    /// An expiry of a durable client's outbox is about to run.
    ExpireBefore,
    /// An expiry ran, durably, and the pool has not told a session of it yet.
    ExpireAfter,
}

impl Stage {
    /// Every stage of a session's work on a binding whose receipt is the transport's, in its
    /// order: what a campaign draws its halts from unless it is told otherwise.
    pub const ALL: [Stage; 5] = [
        Stage::Load,
        Stage::CheckpointBefore,
        Stage::CheckpointAfter,
        Stage::SendBefore,
        Stage::SendAfter,
    ];

    /// Every stage of a server whose client's binding is durable (docs/adr/0109, D10): the
    /// load, the commit that appends to the outbox with the checkpoint, the acknowledgement that
    /// deletes from it, and the expiry. Nothing is handed to the client but by its own poll.
    pub const DURABLE: [Stage; 7] = [
        Stage::Load,
        Stage::CheckpointBefore,
        Stage::CheckpointAfter,
        Stage::AckBefore,
        Stage::AckAfter,
        Stage::ExpireBefore,
        Stage::ExpireAfter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stage::Load => "load",
            Stage::CheckpointBefore => "checkpoint-before",
            Stage::CheckpointAfter => "checkpoint-after",
            Stage::SendBefore => "send-before",
            Stage::SendAfter => "send-after",
            Stage::AckBefore => "ack-before",
            Stage::AckAfter => "ack-after",
            Stage::ExpireBefore => "expire-before",
            Stage::ExpireAfter => "expire-after",
        }
    }

    pub fn from_name(name: &str) -> Option<Stage> {
        Stage::ALL
            .into_iter()
            .chain(Stage::DURABLE)
            .find(|stage| stage.name() == name)
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
    /// What a poll found in the durable client's outbox (docs/adr/0109, D10): the incarnation it
    /// numbers in, and its envelopes by number, oldest first.
    Mailbox {
        epoch: String,
        envelopes: Vec<(u64, Vec<u8>)>,
    },
    /// The pool stopped in order (docs/adr/0109, D10): how many sessions it saved as clean
    /// checkpoints, and how many it could not.
    Stopped { saved: u64, not_saved: u64 },
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
            Report::Mailbox { epoch, envelopes } => {
                let held: Vec<String> = envelopes
                    .iter()
                    .map(|(number, bytes)| format!("{number}:{}", hex(bytes)))
                    .collect();
                format!("mailbox {epoch} {}", held.join(","))
                    .trim_end()
                    .to_string()
            }
            Report::Stopped { saved, not_saved } => format!("stopped {saved} {not_saved}"),
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
            "mailbox" => {
                let epoch = fields.next()?.to_string();
                let envelopes = match fields.next() {
                    None | Some("") => Vec::new(),
                    Some(held) => held
                        .split(',')
                        .map(|entry| {
                            let (number, bytes) = entry.split_once(':')?;
                            Some((number.parse().ok()?, unhex(bytes)?))
                        })
                        .collect::<Option<Vec<_>>>()?,
                };
                Some(Report::Mailbox { epoch, envelopes })
            }
            "stopped" => Some(Report::Stopped {
                saved: fields.next()?.parse().ok()?,
                not_saved: fields.next()?.parse().ok()?,
            }),
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
    /// The durable client's exchange (docs/adr/0109, D10): acknowledge everything numbered at
    /// most `ack` in the incarnation `epoch` (none before the client learned one), and answer
    /// with what its outbox holds ([`Report::Mailbox`]).
    Poll { epoch: Option<String>, ack: u64 },
    /// Stop the pool in order, within this many milliseconds, report [`Report::Stopped`], and
    /// end.
    Stop(u64),
}

impl Command {
    /// The line, without its end.
    pub fn line(&self) -> String {
        match self {
            Command::Envelope(bytes) => format!("envelope {}", hex(bytes)),
            Command::Pause(ms) => format!("pause {ms}"),
            Command::Poll { epoch, ack } => {
                format!("poll {} {ack}", epoch.as_deref().unwrap_or("-"))
            }
            Command::Stop(ms) => format!("stop {ms}"),
        }
    }

    /// The command `line` is, or `None` for a line that is none.
    pub fn parse(line: &str) -> Option<Command> {
        let (word, rest) = line.split_once(' ')?;
        match word {
            "envelope" => unhex(rest).map(Command::Envelope),
            "pause" => rest.parse().ok().map(Command::Pause),
            "poll" => {
                let (epoch, ack) = rest.split_once(' ')?;
                Some(Command::Poll {
                    epoch: (epoch != "-").then(|| epoch.to_string()),
                    ack: ack.parse().ok()?,
                })
            }
            "stop" => rest.parse().ok().map(Command::Stop),
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
    /// The client's binding under `receipt: durable` (docs/adr/0109, D10), when it is: the
    /// client's outbox is kept in the store and the client reads it by polls; `None` hands
    /// every envelope to the client as it is sent.
    pub durable: Option<Durable>,
    /// The machine the server serves: the build of the server it is.
    pub build: Build,
}

/// The machine a server process serves (`--build`): what a deployment changes when it moves a
/// server to its next build (docs/adr/0109, D11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// The ledger, written by hand (`crate::ledger`).
    Ledger,
    /// The first version of the upgraded document, generated (`crate::upgrade`,
    /// tests/server/documents/server_upgrade_1.scxml).
    Upgrade1,
    /// Its next version, which migrates what the first saved (server_upgrade_2.scxml).
    Upgrade2,
}

impl Build {
    /// The value of `--build` that names it.
    pub fn name(self) -> &'static str {
        match self {
            Build::Ledger => "ledger",
            Build::Upgrade1 => "upgrade-1",
            Build::Upgrade2 => "upgrade-2",
        }
    }

    /// The build `name` names, if any.
    pub fn parse(name: &str) -> Option<Build> {
        [Build::Ledger, Build::Upgrade1, Build::Upgrade2]
            .into_iter()
            .find(|build| build.name() == name)
    }
}

/// A durable client binding's bounds (docs/adr/0109, D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durable {
    /// deploy.yaml's `mailbox`: the durable window.
    pub mailbox: u32,
    /// deploy.yaml's `retain_ms`.
    pub retain_ms: u64,
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
            "--build".to_string(),
            self.build.name().to_string(),
        ];
        if let Some(halt) = self.halt {
            args.push("--halt".to_string());
            args.push(halt.to_string());
        }
        if let Some(durable) = self.durable {
            args.push("--durable-mailbox".to_string());
            args.push(durable.mailbox.to_string());
            args.push("--retain-ms".to_string());
            args.push(durable.retain_ms.to_string());
        }
        args
    }

    /// The options `args` (the command line after the program's name) give, or why they give
    /// none. Every option takes a value; every one but `--halt` and the durable binding's pair must be given.
    pub fn from_args(args: &[String]) -> Result<ServerOptions, String> {
        let number = |flag: &str, value: &str| {
            value
                .parse::<u64>()
                .map_err(|_| format!("{flag} takes a count of milliseconds, not '{value}'"))
        };
        let (mut store, mut owner, mut halt) = (None, None, None);
        let (mut idle_ms, mut lease_ttl_ms, mut renew_every_ms) = (None, None, None);
        let (mut mailbox, mut retain_ms, mut build) = (None, None, None);
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
                "--durable-mailbox" => {
                    mailbox = Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| format!("{flag} takes a count, not '{value}'"))?,
                    )
                }
                "--retain-ms" => retain_ms = Some(number(flag, value)?),
                "--build" => {
                    build = Some(
                        Build::parse(value)
                            .ok_or_else(|| format!("--build {value} names no build"))?,
                    )
                }
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
        let durable = match (mailbox, retain_ms) {
            (Some(mailbox), Some(retain_ms)) => Some(Durable { mailbox, retain_ms }),
            (None, None) => None,
            _ => return Err("--durable-mailbox and --retain-ms are given together".to_string()),
        };
        Ok(ServerOptions {
            store: store.ok_or_else(|| required("--store"))?,
            owner: owner.ok_or_else(|| required("--owner"))?,
            idle_ms: idle_ms.ok_or_else(|| required("--idle-ms"))?,
            lease_ttl_ms: lease_ttl_ms.ok_or_else(|| required("--lease-ttl-ms"))?,
            renew_every_ms: renew_every_ms.ok_or_else(|| required("--renew-every-ms"))?,
            halt,
            durable,
            build: build.ok_or_else(|| required("--build"))?,
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
        for command in [
            Command::Envelope(vec![1, 2, 255]),
            Command::Pause(2500),
            Command::Poll {
                epoch: None,
                ack: 0,
            },
            Command::Poll {
                epoch: Some("00112233445566778899aabbccddeeff".to_string()),
                ack: 17,
            },
            Command::Stop(5_000),
        ] {
            assert_eq!(Command::parse(&command.line()), Some(command.clone()));
        }
        assert_eq!(Command::parse("envelope 0"), None);
        assert_eq!(Command::parse("pause soon"), None);
        assert_eq!(Command::parse("poll - many"), None);
    }

    /// docs/adr/0109, D10: what a poll found and what a stop did are read back as written, an
    /// outbox that holds nothing included.
    #[test]
    fn a_mailbox_and_a_stop_are_read_back_as_the_reports_they_write() {
        for report in [
            Report::Mailbox {
                epoch: "00112233445566778899aabbccddeeff".to_string(),
                envelopes: vec![(3, vec![0xa1, 0x00]), (4, vec![0x7f])],
            },
            Report::Mailbox {
                epoch: "00112233445566778899aabbccddeeff".to_string(),
                envelopes: Vec::new(),
            },
            Report::Stopped {
                saved: 12,
                not_saved: 0,
            },
            Report::Halted(Halt {
                stage: Stage::AckAfter,
                at: 2,
            }),
        ] {
            assert_eq!(
                Report::parse(&report.line()),
                Some(report.clone()),
                "{report:?}"
            );
        }
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
            durable: Some(Durable {
                mailbox: 64,
                retain_ms: 60_000,
            }),
            build: Build::Upgrade2,
        };
        assert_eq!(
            ServerOptions::from_args(&options.args()),
            Ok(options.clone())
        );
        let without = ServerOptions {
            halt: None,
            durable: None,
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
