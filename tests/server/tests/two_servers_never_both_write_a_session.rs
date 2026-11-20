// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Two server processes serve one store, and never both write a session (docs/adr/0107, decision
//! D10, its fourth bullet; D6's fencing).
//!
//! A client asks both at random, and asks the other when one says the session is served elsewhere
//! or does not answer. Three times, one server stalls (it stops serving, renewing no lease, for
//! longer than a lease lasts) right after it served a session of its own; the other takes the
//! session over once the stalled server's lease ran out, and the stalled server, when it serves
//! again, is handed a request for that session first, which it runs on the state it still holds
//! and tries to save. The store refuses that save by its fence. That is the case fencing exists
//! for (Kleppmann's analysis of distributed locks, which the session store follows): the clock
//! decides when a key may be taken over, the fence decides whose writes land.
//!
//! What the servers report of every write the store took is then held to the fencing rule: for
//! each session, in the order the store took them, the fences never go back, and each fence is
//! one server's. The store, read when both stopped, holds every answered request once, where its
//! reply said it stands, and every copy of a reply is the first one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

use sce_rust_runtime::json::{self, Value};
use sce_rust_session_store::SqliteStore;
use sce_server_suite::draw::{seed_for, Draw};
use sce_server_suite::ledger::{take_data, taken_by_key, TAKE};
use sce_server_suite::process::{Reported, ServerProcess};
use sce_server_suite::protocol::{Command, Refusal, Report, ServerOptions};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire::{self, Ids, Request, Seen};

/// How long a lease lasts unless it is renewed.
const LEASE_TTL_MS: u64 = 1_500;

/// How long a stalled server stops serving: well past its lease, so that the other takes its
/// sessions over and is answered before the stall ends.
const STALL_MS: u64 = 4_000;

/// How long a request goes unanswered before the client asks again.
const UNANSWERED_MS: u64 = 1_200;

/// How long a request refused for now waits before the client asks again.
const REFUSED_MS: u64 = 200;

/// How long the client waits for any one thing it expects.
const PATIENCE: Duration = Duration::from_secs(30);

/// The sessions the background traffic asks of.
const KEYS: [&str; 6] = ["k0", "k1", "k2", "k3", "k4", "k5"];

/// The two servers' lease owners.
const OWNERS: [&str; 2] = ["a", "b"];

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

/// A request and what came of it.
struct Asked {
    key: String,
    tag: String,
    request: Request,
    /// The server it must go to, every time, if any.
    pinned: Option<usize>,
    /// When to ask again if it is still unanswered.
    again_at: Instant,
    /// The first `Ok` reply, and the server that sent it.
    answer: Option<(usize, Seen)>,
}

/// A checkpoint a server reported the store took.
#[derive(Debug, Clone)]
struct Write {
    server: usize,
    key: String,
    version: u64,
    fence: u64,
}

/// Both servers and the client that asks them.
struct Pair {
    servers: [Option<ServerProcess>; 2],
    asked: Vec<Asked>,
    by_wire: BTreeMap<[u8; 16], usize>,
    writes: Vec<Write>,
    refused: Vec<(usize, String, Refusal)>,
    /// Copies of a reply that differ from the first, by request.
    differing: Vec<String>,
    errors: Vec<String>,
    ids: Ids,
    draw: Draw,
    seed: u64,
    /// The server that is stalled, and until when.
    stalled: Option<(usize, Instant)>,
    background: u64,
}

impl Pair {
    fn start(scratch: &Scratch, seed: u64) -> Pair {
        let start = |owner: &str| {
            let options = ServerOptions {
                store: scratch.store(),
                owner: owner.to_string(),
                // Longer than the run: a session stays in memory, and its lease is renewed, until
                // its server stalls.
                idle_ms: 600_000,
                lease_ttl_ms: LEASE_TTL_MS,
                renew_every_ms: 500,
                halt: None,
                durable: None,
            };
            let server = ServerProcess::start(server(), &options)
                .unwrap_or_else(|error| panic!("the server {owner} could not be started: {error}"));
            if let Err(why) = server.await_ready(PATIENCE) {
                panic!("the server {owner}: {why}");
            }
            server
        };
        // One after the other: the first makes the store's file, the second opens it.
        let first = start(OWNERS[0]);
        let second = start(OWNERS[1]);
        Pair {
            servers: [Some(first), Some(second)],
            asked: Vec::new(),
            by_wire: BTreeMap::new(),
            writes: Vec::new(),
            refused: Vec::new(),
            differing: Vec::new(),
            errors: Vec::new(),
            ids: Ids::new(seed),
            draw: Draw::new(seed),
            seed,
            stalled: None,
            background: 0,
        }
    }

    fn summary(&self) -> String {
        format!(
            "seed {} (SCE_SERVER_SUITE_SEED={} draws it again), {} writes, {} refused, \
             the first errors {:?}",
            self.seed,
            self.seed,
            self.writes.len(),
            self.refused.len(),
            self.errors.iter().take(10).collect::<Vec<_>>()
        )
    }

    /// A server that is not stalled: `preferred` unless it is, the other otherwise.
    fn serving(&self, preferred: usize) -> usize {
        match self.stalled {
            Some((stalled, until)) if stalled == preferred && Instant::now() < until => {
                1 - preferred
            }
            _ => preferred,
        }
    }

    /// Ask the session `key` to take `tag`: of `to` if given (and every time after, if `pinned`),
    /// of a server drawn at random otherwise.
    fn ask(&mut self, key: &str, tag: &str, to: Option<usize>, pinned: bool) -> usize {
        let request = wire::request(&mut self.ids, TAKE, &take_data(key, tag, &[]));
        let index = self.asked.len();
        self.by_wire.insert(request.wire_id, index);
        self.asked.push(Asked {
            key: key.to_string(),
            tag: tag.to_string(),
            request,
            pinned: to.filter(|_| pinned),
            again_at: Instant::now(),
            answer: None,
        });
        let first = match to {
            Some(to) => to,
            None => {
                let drawn = usize::from(self.draw.chance(50));
                self.serving(drawn)
            }
        };
        self.send(first, index);
        index
    }

    fn send(&mut self, to: usize, index: usize) {
        let asked = &mut self.asked[index];
        asked.again_at = Instant::now() + Duration::from_millis(UNANSWERED_MS);
        if let Some(server) = &self.servers[to] {
            server.send(&Command::Envelope(asked.request.bytes.clone()));
        }
    }

    /// Stall the server `which` for [`STALL_MS`].
    fn stall(&mut self, which: usize) {
        if let Some(server) = &self.servers[which] {
            server.send(&Command::Pause(STALL_MS));
        }
        self.stalled = Some((which, Instant::now() + Duration::from_millis(STALL_MS)));
    }

    /// Read what both servers report, ask again what went unanswered, and, with `background`,
    /// ask a session of [`KEYS`] something every 25 milliseconds, for `period`.
    fn pump(&mut self, period: Duration, background: bool) {
        let until = Instant::now() + period;
        let mut next_background = Instant::now();
        while Instant::now() < until {
            self.step(background, &mut next_background);
        }
    }

    fn step(&mut self, background: bool, next_background: &mut Instant) {
        for which in 0..2 {
            let reported = self.servers[which]
                .as_ref()
                .and_then(|server| server.report_within(Duration::from_millis(5)));
            if let Some(reported) = reported {
                self.record(which, reported);
            }
        }
        let now = Instant::now();
        let owed: Vec<usize> = self
            .asked
            .iter()
            .enumerate()
            .filter(|(_, asked)| asked.answer.is_none() && asked.again_at <= now)
            .map(|(index, _)| index)
            .collect();
        for index in owed {
            let to = match self.asked[index].pinned {
                Some(pinned) => pinned,
                None => {
                    let drawn = usize::from(self.draw.chance(50));
                    self.serving(drawn)
                }
            };
            self.send(to, index);
        }
        if background && now >= *next_background {
            *next_background = now + Duration::from_millis(25);
            self.background += 1;
            let key = *self.draw.pick(&KEYS);
            let tag = format!("{key}.b{}", self.background);
            self.ask(key, &tag, None, false);
        }
    }

    fn record(&mut self, from: usize, reported: Reported) {
        match reported.report {
            Report::Sent(bytes) => {
                let Some(seen) = wire::read(bytes) else {
                    self.errors
                        .push(format!("{}: bytes that are no envelope", OWNERS[from]));
                    return;
                };
                let Some(index) = seen
                    .wire_id
                    .and_then(|wire_id| self.by_wire.get(&wire_id).copied())
                else {
                    self.errors
                        .push(format!("{}: a reply to nothing asked", OWNERS[from]));
                    return;
                };
                let asked = &mut self.asked[index];
                if !seen.is_ok_reply() {
                    if asked.answer.is_none() {
                        asked.again_at = Instant::now() + Duration::from_millis(REFUSED_MS);
                    }
                    return;
                }
                let differs = match &asked.answer {
                    None => None,
                    Some((_, first)) => (first.bytes != seen.bytes).then(|| {
                        format!(
                            "{} was answered {:?} and later {:?} by {}",
                            asked.tag, first.data, seen.data, OWNERS[from]
                        )
                    }),
                };
                if asked.answer.is_none() {
                    asked.answer = Some((from, seen));
                }
                if let Some(line) = differs {
                    self.differing.push(line);
                }
            }
            Report::Wrote {
                key,
                version,
                fence,
            } => self.writes.push(Write {
                server: from,
                key,
                version,
                fence,
            }),
            Report::Refused {
                key,
                fence: _,
                refusal,
            } => self.refused.push((from, key, refusal)),
            Report::Error(text) => self.errors.push(format!("{}: {text}", OWNERS[from])),
            // Neither server keeps a durable outbox here, nor is stopped in order.
            Report::Ready | Report::Halted(_) | Report::Mailbox { .. } | Report::Stopped { .. } => {
            }
        }
    }

    /// Pump until the request `index` is answered; the server that answered.
    fn await_answer(&mut self, index: usize, background: bool) -> usize {
        let deadline = Instant::now() + PATIENCE;
        let mut next_background = Instant::now();
        loop {
            if let Some((by, _)) = &self.asked[index].answer {
                return *by;
            }
            assert!(
                Instant::now() < deadline,
                "{} was never answered; {}",
                self.asked[index].tag,
                self.summary()
            );
            self.step(background, &mut next_background);
        }
    }

    /// Pump until the server `which` reports a save of `key` refused by its fence.
    fn await_fenced(&mut self, which: usize, key: &str) {
        let deadline = Instant::now() + PATIENCE;
        let mut next_background = Instant::now();
        while !self
            .refused
            .iter()
            .any(|(by, of, refusal)| *by == which && of == key && *refusal == Refusal::Fence)
        {
            assert!(
                Instant::now() < deadline,
                "the stalled server {} never had a save of {key} refused by its fence; {}",
                OWNERS[which],
                self.summary()
            );
            self.step(false, &mut next_background);
        }
    }

    /// Stop both servers, and read what they had left to report.
    fn stop(&mut self) {
        for which in 0..2 {
            if let Some(server) = self.servers[which].take() {
                for reported in server.stop(PATIENCE) {
                    self.record(which, reported);
                }
            }
        }
    }
}

/// The position a `take` reply says its request stands at.
fn position(data: &str) -> Option<usize> {
    match json::parse(data).ok()?.member("position") {
        Some(Value::Number(digits)) => digits.parse().ok(),
        _ => None,
    }
}

#[test]
fn two_servers_on_one_store_never_both_write_a_session() {
    let seed = seed_for("two_servers_on_one_store_never_both_write_a_session");
    let scratch = Scratch::new("two-servers");
    let mut pair = Pair::start(&scratch, seed);
    pair.pump(Duration::from_secs(2), true);

    let stalls = [0, 1, 0];
    for (round, stalled) in stalls.into_iter().enumerate() {
        let other = 1 - stalled;
        let key = format!("v{round}");
        // A session of the server about to stall: new, so no one else holds its lease.
        let first = pair.ask(&key, &format!("{key}.0"), Some(stalled), true);
        let by = pair.await_answer(first, true);
        assert_eq!(by, stalled, "{}", pair.summary());

        pair.stall(stalled);
        pair.pump(Duration::from_millis(LEASE_TTL_MS + 300), true);
        // The other server takes the session over once the stalled server's lease ran out; until
        // then it says the session is served elsewhere, and the client asks it again.
        let taken_over = pair.ask(&key, &format!("{key}.1"), Some(other), true);
        let by = pair.await_answer(taken_over, true);
        assert_eq!(by, other, "{}", pair.summary());

        // Handed to the stalled server, the first thing it takes when it serves again: it runs it
        // on the state it still holds, under the lease it still believes it has.
        let stale = pair.ask(&key, &format!("{key}.2"), Some(stalled), false);
        pair.await_fenced(stalled, &key);
        pair.await_answer(stale, true);
        pair.pump(Duration::from_millis(500), true);
    }
    let unanswered: Vec<usize> = (0..pair.asked.len())
        .filter(|index| pair.asked[*index].answer.is_none())
        .collect();
    for index in unanswered {
        pair.await_answer(index, false);
    }
    pair.stop();
    let summary = pair.summary();
    eprintln!("two servers: {summary}");

    // The fencing rule, over every write the store took.
    let mut by_key: BTreeMap<&str, Vec<&Write>> = BTreeMap::new();
    for write in &pair.writes {
        by_key.entry(write.key.as_str()).or_default().push(write);
    }
    let mut owner_of_fence: BTreeMap<u64, usize> = BTreeMap::new();
    for (key, writes) in &mut by_key {
        writes.sort_by_key(|write| write.version);
        for pair_of in writes.windows(2) {
            let (earlier, later) = (pair_of[0], pair_of[1]);
            assert!(
                later.version > earlier.version,
                "two writes of {key} took one version: {earlier:?} and {later:?}; {summary}"
            );
            assert!(
                later.fence >= earlier.fence,
                "{key} was written under fence {} after it was written under fence {}: both \
                 {} and {} wrote it; {summary}",
                later.fence,
                earlier.fence,
                OWNERS[earlier.server],
                OWNERS[later.server]
            );
        }
        for write in writes.iter() {
            let owner = *owner_of_fence.entry(write.fence).or_insert(write.server);
            assert_eq!(
                owner, write.server,
                "the fence {} wrote for both servers; {summary}",
                write.fence
            );
        }
    }
    let servers_writing: BTreeSet<usize> = pair.writes.iter().map(|write| write.server).collect();
    assert_eq!(
        servers_writing.len(),
        2,
        "only one server ever wrote, so nothing was taken over; {summary}"
    );

    assert!(
        pair.differing.is_empty(),
        "a reply was answered again with another: {:?}; {summary}",
        pair.differing
    );
    let store = SqliteStore::open(scratch.store()).expect("the store both servers kept");
    let keys: BTreeSet<String> = pair.asked.iter().map(|asked| asked.key.clone()).collect();
    let taken = taken_by_key(&store, keys).unwrap_or_else(|error| panic!("{error}; {summary}"));
    let mut asked_of: BTreeMap<&str, usize> = BTreeMap::new();
    for asked in &pair.asked {
        *asked_of.entry(asked.key.as_str()).or_default() += 1;
        let Some((_, answer)) = &asked.answer else {
            panic!("{} was never answered; {summary}", asked.tag);
        };
        let at: Vec<usize> = taken[&asked.key]
            .iter()
            .enumerate()
            .filter(|(_, tag)| **tag == asked.tag)
            .map(|(at, _)| at + 1)
            .collect();
        assert_eq!(
            at.len(),
            1,
            "{} is held {} times in the store; {summary}",
            asked.tag,
            at.len()
        );
        assert_eq!(
            position(&answer.data),
            Some(at[0]),
            "the reply to {} named another position than the store holds it at; {summary}",
            asked.tag
        );
    }
    for (key, count) in asked_of {
        assert_eq!(
            taken[key].len(),
            count,
            "the store holds {:?} for {key}, not the {count} requests answered; {summary}",
            taken[key]
        );
    }
}
