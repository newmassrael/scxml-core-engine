// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A server killed again and again while its client keeps asking.
//!
//! A [`Campaign`] runs one server process after another on one store. Each life but the last ends
//! in `SIGKILL`: at a [`Halt`] the campaign chose (the stages in turn, each at a count it drew),
//! or, when the life does not reach its halt in a span drawn for it, wherever the server is on the
//! clock. The last life is not killed: it runs until the scenario's work is done, or its patience
//! ends, and is stopped by closing its input.
//!
//! The client is the campaign itself. It sends each request it was given, at the plan's pace, and,
//! at every start of a server, sends again every request it sent and saw no answer to (a client
//! whose exchange failed sends again) and some it saw answered (a client that lost the answer
//! sends again too, which is the retry SCE_MESH.md section 10.5 answers with the first reply). A
//! request refused for now (`Unavailable`) is sent again a moment later. Everything the server
//! sent the client is kept, with the moment it was read: what a scenario asserts on.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use sce_rust_runtime::json::{self, Value};

use crate::draw::Draw;
use crate::ledger::{REMINDED, TAKE};
use crate::process::{Reported, ServerProcess};
use crate::protocol::{Command, Halt, Report, ServerOptions, Stage};
use crate::wire::{self, Ids, Request, Seen};

/// How long a server may take to open its store and say it serves.
const READY_PATIENCE: Duration = Duration::from_secs(30);

/// How long a stopped server may take to finish the commands it holds.
const STOP_PATIENCE: Duration = Duration::from_secs(30);

/// How often the campaign sends its next requests and looks at what came back.
const STEP: Duration = Duration::from_millis(10);

/// How long a request refused for now waits before it is sent again.
const REFUSED_AGAIN: Duration = Duration::from_millis(200);

/// How a campaign runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// How many lives end in `SIGKILL`; one more, the last, is stopped.
    pub kills: u64,
    /// The shortest and the longest a killed life lasts when it does not reach its halt first, in
    /// milliseconds from the moment it said it serves.
    pub life_ms: (u64, u64),
    /// A halt is drawn at a count from 1 to this.
    pub halt_reach: u64,
    /// How many requests never sent are sent at each step of 10 milliseconds.
    pub pace: usize,
    /// How often, in a hundred, a request seen answered is sent again at a server's start.
    pub retry_answered_percent: u64,
    /// How long the last life may take to finish the scenario's work.
    pub patience: Duration,
}

/// A request of the campaign and what came of it.
#[derive(Debug, Clone)]
pub struct Asked {
    pub key: String,
    pub tag: String,
    pub request: Request,
    /// Every reply that ended it well, in the order read: the first, and each copy after it.
    pub answers: Vec<Seen>,
    /// How many times it was refused for now.
    pub refusals: u64,
    /// How many times it was sent.
    pub sends: u64,
}

/// A delayed send the client received.
#[derive(Debug, Clone)]
pub struct Delivery {
    /// When the client read it, in milliseconds since the Unix epoch.
    pub read_ms: u64,
    /// The session that sent it.
    pub key: String,
    /// The due its session held for it, as it says.
    pub due_ms: u64,
    /// When its session sent it, as it says.
    pub sent_ms: u64,
    pub seen: Seen,
}

/// The client of a server whose binding is durable (docs/adr/0109, D10): it reads its outbox by
/// polls, acknowledging what it took in, goes away for a while now and then, and now and then
/// loses a response, as a client whose exchange failed does.
#[derive(Debug, Clone, Default)]
struct DurableClient {
    /// The incarnation the outbox numbers in, as the last poll the client took said.
    epoch: Option<String>,
    /// The highest number the client took in, in that incarnation: what it acknowledges.
    ack: u64,
    /// How often, in a hundred, the client goes away at a step, and for how long, in
    /// milliseconds.
    absent_percent: u64,
    absent_ms: (u64, u64),
    /// Until when it is away.
    absent_until: Option<Instant>,
    /// How often, in a hundred, a poll's response is lost before the client takes it in.
    lost_percent: u64,
    /// How many responses were lost, and how many times the client went away.
    lost_responses: u64,
    absences: u64,
    /// How many incarnations the client saw its outbox number in.
    incarnations: u64,
}

/// A server killed again and again while its client keeps asking. See the module documentation.
pub struct Campaign {
    program: PathBuf,
    options: ServerOptions,
    seed: u64,
    draw: Draw,
    ids: Ids,
    asked: Vec<Asked>,
    by_wire: HashMap<[u8; 16], usize>,
    owed: VecDeque<(Instant, usize)>,
    deliveries: BTreeMap<String, Vec<Delivery>>,
    strays: Vec<Seen>,
    halts: Vec<Halt>,
    errors: Vec<String>,
    kills: u64,
    resent_answered: u64,
    /// The stages a life's halt is drawn from.
    stages: &'static [Stage],
    /// The client of a durable binding, when the server keeps one.
    durable: Option<DurableClient>,
}

impl Campaign {
    /// A campaign of the server `program` started with `options` (whose halt it sets for each
    /// life), drawing every choice from `seed`.
    pub fn new(program: &Path, options: ServerOptions, seed: u64) -> Campaign {
        Campaign {
            program: program.to_path_buf(),
            options,
            seed,
            draw: Draw::new(seed),
            ids: Ids::new(seed),
            asked: Vec::new(),
            by_wire: HashMap::new(),
            owed: VecDeque::new(),
            deliveries: BTreeMap::new(),
            strays: Vec::new(),
            halts: Vec::new(),
            errors: Vec::new(),
            kills: 0,
            resent_answered: 0,
            stages: &Stage::ALL,
            durable: None,
        }
    }

    /// Make the campaign's client that of a durable binding (docs/adr/0109, D10), for a server
    /// started with `ServerOptions::durable`: it reads the server's sends by polls, each
    /// acknowledging what it took in; at each step it goes away with a chance of
    /// `absent_percent` in a hundred, for between `absent_ms.0` and `absent_ms.1` milliseconds,
    /// polling nothing meanwhile; and it loses a poll's response before taking it in with a chance
    /// of `lost_percent`. A life's halt is drawn from [`Stage::DURABLE`]: the commit, the
    /// acknowledgement and the expiry.
    pub fn durable(&mut self, absent_percent: u64, absent_ms: (u64, u64), lost_percent: u64) {
        self.stages = &Stage::DURABLE;
        self.durable = Some(DurableClient {
            absent_percent,
            absent_ms,
            lost_percent,
            ..DurableClient::default()
        });
    }

    /// How many poll responses the durable client lost, how many times it went away, and how
    /// many incarnations it saw its outbox number in.
    pub fn durable_counts(&self) -> (u64, u64, u64) {
        self.durable.as_ref().map_or((0, 0, 0), |client| {
            (client.lost_responses, client.absences, client.incarnations)
        })
    }

    /// The campaign's own draw, for the scenario's choices of what to ask.
    pub fn draw(&mut self) -> &mut Draw {
        &mut self.draw
    }

    /// Add a `take` request for the session `key` with `tag` and `data`, sent in the order added.
    pub fn ask(&mut self, key: &str, tag: &str, data: &str) {
        let request = wire::request(&mut self.ids, TAKE, data);
        self.by_wire.insert(request.wire_id, self.asked.len());
        self.asked.push(Asked {
            key: key.to_string(),
            tag: tag.to_string(),
            request,
            answers: Vec::new(),
            refusals: 0,
            sends: 0,
        });
    }

    /// Run the campaign by `plan` until `finished` holds of it in the last life, or that life's
    /// patience ends. An error is a server that could not be started or never said it serves;
    /// what the server did is for the scenario to judge.
    pub fn run(&mut self, plan: &Plan, finished: impl Fn(&Campaign) -> bool) -> Result<(), String> {
        let mut sent_new = 0;
        for life in 0..=plan.kills {
            let last = life == plan.kills;
            let halt = if last {
                None
            } else {
                self.draw_halt(life, plan)
            };
            let options = ServerOptions {
                halt,
                ..self.options.clone()
            };
            let server = ServerProcess::start(&self.program, &options)
                .map_err(|error| format!("the server could not be started: {error}"))?;
            let early = server.await_ready(READY_PATIENCE)?;
            self.record_all(early);
            let span = if last {
                plan.patience
            } else {
                Duration::from_millis(self.draw.between(plan.life_ms.0, plan.life_ms.1))
            };
            let ends = Instant::now() + span;
            for index in 0..sent_new {
                let again = if self.asked[index].answers.is_empty() {
                    true
                } else if self.draw.chance(plan.retry_answered_percent) {
                    self.resent_answered += 1;
                    true
                } else {
                    false
                };
                if again {
                    self.send(&server, index);
                }
            }
            let mut next_step = Instant::now();
            loop {
                let now = Instant::now();
                if now >= next_step {
                    let upto = (sent_new + plan.pace).min(self.asked.len());
                    for index in sent_new..upto {
                        self.send(&server, index);
                    }
                    sent_new = upto;
                    self.poll(&server, now);
                    next_step = now + STEP;
                }
                while let Some(&(due, index)) = self.owed.front() {
                    if due > now {
                        break;
                    }
                    self.owed.pop_front();
                    if self.asked[index].answers.is_empty() {
                        self.send(&server, index);
                    }
                }
                let mut halted = false;
                if let Some(reported) = server.report_within(STEP) {
                    halted = matches!(reported.report, Report::Halted(_));
                    self.record(reported);
                }
                if halted {
                    break;
                }
                if last && sent_new == self.asked.len() && finished(self) {
                    break;
                }
                if Instant::now() >= ends {
                    break;
                }
            }
            let rest = if last {
                server.stop(STOP_PATIENCE)
            } else {
                self.kills += 1;
                server.kill()
            };
            self.record_all(rest);
        }
        Ok(())
    }

    /// The seed every choice was drawn from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Every request, in the order added.
    pub fn asked(&self) -> &[Asked] {
        &self.asked
    }

    /// Every delayed send the client received, by the tag it carries.
    pub fn deliveries(&self) -> &BTreeMap<String, Vec<Delivery>> {
        &self.deliveries
    }

    /// What the server sent that the client asked nothing for and expected nothing of.
    pub fn strays(&self) -> &[Seen] {
        &self.strays
    }

    /// Every halt a life reached and was killed at.
    pub fn halts(&self) -> &[Halt] {
        &self.halts
    }

    /// What the servers told their hosts, and anything they wrote that the client could not read.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// How many lives were killed.
    pub fn kills(&self) -> u64 {
        self.kills
    }

    /// How many times a request seen answered was sent again.
    pub fn resent_answered(&self) -> u64 {
        self.resent_answered
    }

    /// Whether every request was answered.
    pub fn all_answered(&self) -> bool {
        self.asked.iter().all(|asked| !asked.answers.is_empty())
    }

    /// A line for a failed assertion: the seed that draws the run again, what was killed where,
    /// and the first things the servers reported.
    pub fn summary(&self) -> String {
        let halts: Vec<String> = self.halts.iter().map(Halt::to_string).collect();
        let errors: Vec<&String> = self.errors.iter().take(10).collect();
        format!(
            "seed {} ({}={} draws it again), {} kills, halts reached {halts:?}, \
             {} errors reported, the first {errors:?}",
            self.seed,
            crate::draw::SEED_VARIABLE,
            self.seed,
            self.kills,
            self.errors.len(),
        )
    }

    /// The halt of the life `life`: the campaign's stages in turn, so that a campaign of as many
    /// kills as it has stages draws every one, then a life killed on the clock alone; each at a
    /// count drawn up to the plan's reach.
    fn draw_halt(&mut self, life: u64, plan: &Plan) -> Option<Halt> {
        let turn = usize::try_from(life % (self.stages.len() as u64 + 1)).unwrap_or(0);
        let stage = *self.stages.get(turn)?;
        Some(Halt {
            stage,
            at: self.draw.between(1, plan.halt_reach.max(1)),
        })
    }

    fn send(&mut self, server: &ServerProcess, index: usize) {
        let asked = &mut self.asked[index];
        asked.sends += 1;
        server.send(&Command::Envelope(asked.request.bytes.clone()));
    }

    fn record_all(&mut self, reports: Vec<Reported>) {
        for reported in reports {
            self.record(reported);
        }
    }

    fn record(&mut self, reported: Reported) {
        match reported.report {
            Report::Sent(bytes) => self.received(reported.at_ms, bytes),
            Report::Mailbox { epoch, envelopes } => self.polled(reported.at_ms, epoch, envelopes),
            Report::Halted(halt) => self.halts.push(halt),
            Report::Error(text) => self.errors.push(text),
            Report::Ready
            | Report::Wrote { .. }
            | Report::Refused { .. }
            | Report::Stopped { .. } => {}
        }
    }

    /// The client took in `bytes`, an envelope the server sent it, read at `at_ms`.
    fn received(&mut self, at_ms: u64, bytes: Vec<u8>) {
        let Some(seen) = wire::read(bytes) else {
            self.errors
                .push("the server sent bytes that are no envelope".to_string());
            return;
        };
        if seen.event == REMINDED {
            self.delivered(at_ms, seen);
            return;
        }
        let index = seen
            .wire_id
            .and_then(|wire_id| self.by_wire.get(&wire_id).copied());
        let Some(index) = index else {
            self.strays.push(seen);
            return;
        };
        if seen.is_ok_reply() {
            self.asked[index].answers.push(seen);
        } else {
            self.asked[index].refusals += 1;
            self.owed.push_back((Instant::now() + REFUSED_AGAIN, index));
        }
    }

    /// Poll the durable client's outbox (docs/adr/0109, D10), acknowledging what it took in,
    /// unless the client is away; at `now` it may go away.
    fn poll(&mut self, server: &ServerProcess, now: Instant) {
        let Some((absent_percent, absent_ms)) = self
            .durable
            .as_ref()
            .map(|client| (client.absent_percent, client.absent_ms))
        else {
            return;
        };
        let away = self
            .durable
            .as_ref()
            .and_then(|client| client.absent_until)
            .is_some_and(|until| now < until);
        if away {
            return;
        }
        if self.draw.chance(absent_percent) {
            let span = Duration::from_millis(self.draw.between(absent_ms.0, absent_ms.1));
            if let Some(client) = self.durable.as_mut() {
                client.absent_until = Some(now + span);
                client.absences += 1;
            }
            return;
        }
        if let Some(client) = &self.durable {
            server.send(&Command::Poll {
                epoch: client.epoch.clone(),
                ack: client.ack,
            });
        }
    }

    /// A poll found `envelopes` in the outbox, numbered in the incarnation `epoch`, read at
    /// `at_ms`: unless the response is lost, the client takes in each it has not taken in yet,
    /// in their order, and acknowledges them at its next poll. A response of another incarnation
    /// than the one it knew is an outbox that numbers anew.
    fn polled(&mut self, at_ms: u64, epoch: String, envelopes: Vec<(u64, Vec<u8>)>) {
        let lost_percent = self
            .durable
            .as_ref()
            .map_or(0, |client| client.lost_percent);
        if self.durable.is_none() {
            self.errors
                .push("a mailbox reached a client that polls none".to_string());
            return;
        }
        if !envelopes.is_empty() && self.draw.chance(lost_percent) {
            if let Some(client) = self.durable.as_mut() {
                client.lost_responses += 1;
            }
            return;
        }
        let mut fresh = Vec::new();
        if let Some(client) = self.durable.as_mut() {
            if client.epoch.as_deref() != Some(epoch.as_str()) {
                client.epoch = Some(epoch);
                client.ack = 0;
                client.incarnations += 1;
            }
            for (number, bytes) in envelopes {
                if number <= client.ack {
                    continue;
                }
                client.ack = number;
                fresh.push(bytes);
            }
        }
        for bytes in fresh {
            self.received(at_ms, bytes);
        }
    }

    fn delivered(&mut self, read_ms: u64, seen: Seen) {
        let data = json::parse(&seen.data).unwrap_or(Value::Null);
        let text = |name: &str| match data.member(name) {
            Some(Value::Text(text)) => Some(text.clone()),
            _ => None,
        };
        let number = |name: &str| match data.member(name) {
            Some(Value::Number(digits)) => digits.parse::<u64>().ok(),
            _ => None,
        };
        let (Some(tag), Some(key), Some(due_ms), Some(sent_ms)) = (
            text("tag"),
            text("key"),
            number("due_ms"),
            number("sent_ms"),
        ) else {
            self.strays.push(seen);
            return;
        };
        self.deliveries.entry(tag).or_default().push(Delivery {
            read_ms,
            key,
            due_ms,
            sent_ms,
            seen,
        });
    }
}
