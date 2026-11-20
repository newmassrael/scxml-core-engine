// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The suite's server process: the ledger machine served as a keyed pool on the Rust host core,
//! its sessions kept in the reference store, its one client's envelopes carried over its standard
//! streams (`sce_server_suite::protocol`).
//!
//! It is the host a deployment would write, in the smallest form: it hands the pool each envelope
//! the client sends and runs the session it names, and between commands it wakes the sessions
//! whose delayed sends came due ([`Pool::due`]), drops the idle ones ([`Pool::evict_idle`]) and
//! renews its leases ([`Pool::renew_leases`]), sleeping no longer than [`Pool::next_wake_ms`]
//! allows. Besides, it tells its parent every checkpoint the store took or refused, and stops at
//! the halt it was given, so that the parent kills it there.

#![forbid(unsafe_code)]

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{Transport, TransportFailure};
use sce_rust_mesh::pool::{Pool, PoolConfig};
use sce_rust_session_store::{
    AcquireError, Append, Appended, Change, Conflict, Due, Expiry, Failure, Lease, LeaseError,
    Note, Outbox, SessionStore, SqliteStore, Stored, Write, WriteError, Written,
};
use sce_server_suite::clock::{wall_ms, WallClock};
use sce_server_suite::ledger::{self, Ledgers, CLIENT, PIPE};
use sce_server_suite::protocol::{Command, Halt, Refusal, Report, ServerOptions, Stage};

/// The longest the server waits for a command before it looks at what came due.
const POLL: Duration = Duration::from_millis(20);

/// The most commands it takes between two looks at what came due.
const BATCH: usize = 256;

/// The pool this process serves.
type Served = Pool<Ledgers, Lines, WallClock>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match ServerOptions::from_args(&args) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("sce-suite-server: {why}");
            return ExitCode::from(2);
        }
    };
    let store = match SqliteStore::open(&options.store) {
        Ok(store) => store,
        Err(failure) => {
            eprintln!("sce-suite-server: {failure}");
            return ExitCode::from(1);
        }
    };
    // docs/adr/0109, D10: a durable client's binding keeps its outbox in the store, which the
    // client reads by its polls; any other has every envelope handed to it as it is sent.
    let binding = match options.durable {
        Some(durable) => ledger::durable_https(durable.mailbox, durable.retain_ms),
        None => PIPE,
    };
    let pool = Pool::new(
        ledger::machine(binding, options.idle_ms),
        PoolConfig {
            owner: options.owner.clone(),
            lease_ttl_ms: options.lease_ttl_ms,
            renew_every_ms: options.renew_every_ms,
        },
        Ledgers::default(),
        Arc::new(Observed {
            inner: store,
            halt: options.halt,
            loads: AtomicU64::new(0),
            puts: AtomicU64::new(0),
            acks: AtomicU64::new(0),
            expiries: AtomicU64::new(0),
        }),
        Lines {
            halt: options.halt,
            sends: 0,
        },
        WallClock::default(),
    );
    let pool = match pool {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("sce-suite-server: {error:?}");
            return ExitCode::from(1);
        }
    };
    pool.peer_ready(CLIENT);
    report(&Report::Ready);
    serve(&pool, &read_commands());
    ExitCode::SUCCESS
}

/// The lines of standard input, read by a thread of their own; the channel ends when the input
/// does.
fn read_commands() -> Receiver<String> {
    let (lines, commands) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else {
                return;
            };
            if lines.send(line).is_err() {
                return;
            }
        }
    });
    commands
}

/// Serve until the input ends.
fn serve(pool: &Served, commands: &Receiver<String>) {
    loop {
        let wait = pool.next_wake_ms().map_or(POLL, |at| {
            Duration::from_millis(at.saturating_sub(wall_ms())).min(POLL)
        });
        let first = match commands.recv_timeout(wait) {
            Ok(line) => Some(line),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut paused = false;
        for line in first.into_iter().chain(commands.try_iter().take(BATCH)) {
            match Command::parse(&line) {
                Some(Command::Envelope(bytes)) => {
                    if let Some(key) = pool.receive(CLIENT, bytes) {
                        pool.run(&key);
                    }
                }
                Some(Command::Pause(ms)) => {
                    std::thread::sleep(Duration::from_millis(ms));
                    paused = true;
                    break;
                }
                Some(Command::Poll { epoch, ack }) => poll(pool, epoch, ack),
                Some(Command::Stop(ms)) => {
                    let stopped = pool.stop(Instant::now() + Duration::from_millis(ms));
                    for (key, reason) in &stopped.not_saved {
                        report(&Report::Error(format!("not saved: {key}: {reason}")));
                    }
                    report(&Report::Stopped {
                        saved: stopped.saved.len() as u64,
                        not_saved: stopped.not_saved.len() as u64,
                    });
                    return;
                }
                None => report(&Report::Error(format!(
                    "a command this server does not read: {line}"
                ))),
            }
        }
        // A server that stalled takes the next command before anything else: it renews no lease
        // first, as a process that does not know it stalled renews none.
        if paused {
            continue;
        }
        for key in pool.due() {
            pool.run(&key);
        }
        // docs/adr/0109, D10: what a durable client was kept longer than its binding's
        // `retain_ms` expires, and each sender in memory is run to be told.
        for key in pool.expire() {
            pool.run(&key);
        }
        pool.evict_idle();
        for key in pool.renew_leases() {
            pool.run(&key);
        }
        for error in pool.take_errors() {
            report(&Report::Error(format!("{error:?}")));
        }
    }
}

/// The durable client's exchange (docs/adr/0109, D10), as the HTTPS server makes it: its outbox
/// opened (in an incarnation drawn now when the store holds none), what `ack` covers in `epoch`
/// deleted in the store, and what the outbox holds reported.
fn poll(pool: &Served, epoch: Option<String>, ack: u64) {
    let Some(outboxes) = pool.outboxes() else {
        report(&Report::Error(
            "a poll reached a server that keeps no outbox".to_string(),
        ));
        return;
    };
    let now = wall_ms();
    let drawn = format!("{now:016x}{:016x}", u64::from(std::process::id()));
    let incarnation = match outboxes.open(CLIENT, &drawn, now) {
        Ok(incarnation) => incarnation,
        Err(failure) => {
            report(&Report::Error(format!(
                "the outbox could not be opened: {failure}"
            )));
            return;
        }
    };
    if ack > 0 && epoch.as_deref() == Some(incarnation.as_str()) {
        if let Err(failure) = outboxes.acknowledge(CLIENT, &incarnation, ack, now) {
            report(&Report::Error(format!(
                "the acknowledgement was not kept: {failure}"
            )));
            return;
        }
    }
    let (epoch, envelopes) = outboxes
        .entries(CLIENT)
        .unwrap_or((incarnation, Vec::new()));
    report(&Report::Mailbox { epoch, envelopes });
}

/// Write `report` to standard output as one line.
fn report(report: &Report) {
    let mut out = std::io::stdout().lock();
    // A parent that stopped reading has ended; there is no one left to tell.
    let _ = writeln!(out, "{}", report.line()).and_then(|()| out.flush());
}

/// Stop at `halt` if this is it: say so, and wait to be killed.
fn halt_at(halt: Option<Halt>, stage: Stage, at: u64) {
    let here = Halt { stage, at };
    if halt == Some(here) {
        report(&Report::Halted(here));
        loop {
            std::thread::sleep(Duration::from_secs(3_600));
        }
    }
}

/// The reference store, watched: every checkpoint it takes or refuses is reported, and the
/// server stops at its halt when a load or a checkpoint reaches it.
struct Observed {
    inner: SqliteStore,
    halt: Option<Halt>,
    /// How many checkpoints were read: one for each session loaded.
    loads: AtomicU64,
    /// How many checkpoints were written.
    puts: AtomicU64,
    /// How many acknowledgements of the durable client's outbox were made, and expiries run
    /// (docs/adr/0109, D10).
    acks: AtomicU64,
    expiries: AtomicU64,
}

impl SessionStore for Observed {
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        let n = self.loads.fetch_add(1, Ordering::SeqCst) + 1;
        halt_at(self.halt, Stage::Load, n);
        self.inner.get(key)
    }

    /// A checkpoint (a write that keeps the session) is reported and may halt; a removal is
    /// passed on as it is.
    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        if matches!(write.change, Change::Remove) {
            return self.inner.write(key, write);
        }
        let fence = write.fence;
        let n = self.puts.fetch_add(1, Ordering::SeqCst) + 1;
        halt_at(self.halt, Stage::CheckpointBefore, n);
        let written = self.inner.write(key, write);
        match &written {
            Ok(done) => {
                report(&Report::Wrote {
                    key: key.to_string(),
                    version: done.version,
                    fence,
                });
                halt_at(self.halt, Stage::CheckpointAfter, n);
            }
            Err(WriteError::Conflict(conflict)) => report(&Report::Refused {
                key: key.to_string(),
                fence,
                refusal: match conflict {
                    Conflict::Fence { .. } => Refusal::Fence,
                    Conflict::Version { .. } => Refusal::Version,
                },
            }),
            Err(WriteError::Failed(_)) => {}
        }
        written
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        self.inner.due_before(ms, limit)
    }

    fn indexed(&self, key: &str) -> String {
        self.inner.indexed(key)
    }

    /// Not a load: the pool names what came due before it takes the key's lease and loads it.
    fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
        self.inner.key_of(indexed)
    }

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        self.inner.indexed_after(after, limit)
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        self.inner.acquire(key, owner, ttl_ms, now_ms)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        self.inner.renew(key, fence, ttl_ms, now_ms)
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        self.inner.release(key, fence)
    }

    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        self.inner.append(appends, now_ms)
    }

    fn outbox(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        self.inner.outbox(client)
    }

    fn open_outbox(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<Outbox, Failure> {
        self.inner.open_outbox(client, incarnation, now_ms)
    }

    /// An acknowledgement of the durable client may halt before or after it deletes, durably,
    /// what it covers.
    fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        let n = self.acks.fetch_add(1, Ordering::SeqCst) + 1;
        halt_at(self.halt, Stage::AckBefore, n);
        let deleted = self.inner.acknowledge(client, incarnation, through, now_ms);
        if deleted.is_ok() {
            halt_at(self.halt, Stage::AckAfter, n);
        }
        deleted
    }

    /// An expiry may halt before or after it ran, durably.
    fn expire(&self, client: &str, before_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let n = self.expiries.fetch_add(1, Ordering::SeqCst) + 1;
        halt_at(self.halt, Stage::ExpireBefore, n);
        let expiry = self.inner.expire(client, before_ms, limit);
        if expiry.is_ok() {
            halt_at(self.halt, Stage::ExpireAfter, n);
        }
        expiry
    }

    fn notes(&self, key: &str) -> Result<Vec<Note>, Failure> {
        self.inner.notes(key)
    }
}

/// The client's side of the pool's transport: every envelope is a line of standard output, and
/// the server stops at its halt when a send reaches it.
struct Lines {
    halt: Option<Halt>,
    sends: u64,
}

impl Transport for Lines {
    fn transmit(&mut self, _peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        self.sends += 1;
        halt_at(self.halt, Stage::SendBefore, self.sends);
        report(&Report::Sent(bytes.to_vec()));
        halt_at(self.halt, Stage::SendAfter, self.sends);
        Ok(())
    }
}
