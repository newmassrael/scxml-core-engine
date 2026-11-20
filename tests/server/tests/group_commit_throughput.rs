// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How many inputs a keyed pool takes in a second on the reference store, and how many
//! transactions it pays for them, as the clients that send at once grow (docs/adr/0109, D15).
//!
//! A measurement, not a gate: it is ignored by default, its numbers depend on the machine and its
//! disk, and they are recorded in docs/SCE_SERVER_DEPLOY.md, section 3, "Group commit", where the
//! procedure to run it is written. It asserts only what holds on any machine: every input is
//! answered, and no input costs more than one transaction.
//!
//! Each client is a thread of its own with a key of its own, which hands the pool its requests one
//! after another and runs the session they name, as a server's per-client delivery does
//! (SCE_MESH.md section 22.8). One client is the serial baseline: each of its checkpoints is a
//! transaction of its own, synced alone.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use sce_rust_mesh::pool::{Pool, PoolConfig};
use sce_rust_session_store::{
    AcquireError, Batched, Decided, Due, Failure, Lease, LeaseError, SessionStore, SqliteStore,
    Stored, Write, WriteError, Written,
};
use sce_server_suite::clock::WallClock;
use sce_server_suite::ledger::{self, key_data, Ledgers, CLIENT, JOIN, PIPE};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire::{self, Ids, Recording};

/// The environment variable the counts of clients are read from: numbers separated by commas.
const CLIENTS_VARIABLE: &str = "SCE_SERVER_SUITE_CLIENTS";

/// The environment variable the requests each client sends are read from.
const REQUESTS_VARIABLE: &str = "SCE_SERVER_SUITE_REQUESTS";

/// The reference store, counting the transactions it is asked for and the writes they hold.
struct Counted {
    inner: SqliteStore,
    transactions: AtomicU64,
    writes: AtomicU64,
}

impl SessionStore for Counted {
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        self.inner.get(key)
    }

    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        self.transactions.fetch_add(1, Ordering::SeqCst);
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.inner.write(key, write)
    }

    fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
        self.transactions.fetch_add(1, Ordering::SeqCst);
        self.writes.fetch_add(batch.len() as u64, Ordering::SeqCst);
        self.inner.write_many(batch)
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        self.inner.due_before(ms, limit)
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
}

/// What one run measured.
struct Measured {
    clients: u64,
    inputs: u64,
    transactions: u64,
    seconds: f64,
}

/// `clients` clients, each sending `requests` requests to a key of its own, against a pool on a
/// reference store of its own.
fn run(clients: u64, requests: u64) -> Measured {
    let scratch = Scratch::new(&format!("throughput-{clients}"));
    let store = Arc::new(Counted {
        inner: SqliteStore::open(scratch.store()).expect("the reference store"),
        transactions: AtomicU64::new(0),
        writes: AtomicU64::new(0),
    });
    let recording = Recording::default();
    let pool = Pool::new(
        ledger::machine(PIPE, 600_000),
        PoolConfig {
            owner: "server".to_string(),
            lease_ttl_ms: 600_000,
            renew_every_ms: 300_000,
        },
        Ledgers::default(),
        Arc::clone(&store) as Arc<dyn SessionStore>,
        recording.clone(),
        WallClock::default(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    pool.peer_ready(CLIENT);
    let pool = Arc::new(pool);

    let started = Instant::now();
    let senders: Vec<_> = (0..clients)
        .map(|n| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || {
                let key = format!("k{n}");
                let mut ids = Ids::new(n + 1);
                for _ in 0..requests {
                    let request = wire::request(&mut ids, JOIN, &key_data(&key));
                    let key = pool
                        .receive(CLIENT, request.bytes)
                        .expect("the request names its session");
                    pool.run(&key);
                }
            })
        })
        .collect();
    for sender in senders {
        sender.join().expect("no client panicked");
    }
    let seconds = started.elapsed().as_secs_f64();

    let inputs = clients * requests;
    let answered = recording.take().len() as u64;
    assert_eq!(answered, inputs, "every input is answered");
    assert!(
        pool.take_errors().is_empty(),
        "the pool met an error it could not act on"
    );
    let transactions = store.transactions.load(Ordering::SeqCst);
    let writes = store.writes.load(Ordering::SeqCst);
    assert!(
        transactions <= writes,
        "{transactions} transactions for {writes} writes"
    );
    Measured {
        clients,
        inputs,
        transactions,
        seconds,
    }
}

/// The counts `variable` names, or `default` when it names none.
fn counts_from(variable: &str, default: &[u64]) -> Vec<u64> {
    match std::env::var(variable) {
        Ok(text) => text
            .split(',')
            .map(|count| {
                count
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("{variable}={text} is not a list of counts"))
            })
            .collect(),
        Err(_) => default.to_vec(),
    }
}

/// Measure, for each count of clients, the inputs taken in a second and the inputs a transaction
/// held, and print one line per count. Run it as docs/SCE_SERVER_DEPLOY.md section 3 says.
#[test]
#[ignore = "a measurement whose numbers are recorded in docs/SCE_SERVER_DEPLOY.md, not a gate"]
fn group_commit_throughput() {
    let clients = counts_from(CLIENTS_VARIABLE, &[1, 4, 16, 64]);
    let requests = sce_server_suite::count_from(REQUESTS_VARIABLE, 200);
    println!("clients  inputs  transactions  inputs/transaction  inputs/s");
    for count in clients {
        let measured = run(count, requests);
        println!(
            "{:>7}  {:>6}  {:>12}  {:>18.2}  {:>8.0}",
            measured.clients,
            measured.inputs,
            measured.transactions,
            measured.inputs as f64 / measured.transactions.max(1) as f64,
            measured.inputs as f64 / measured.seconds
        );
    }
}
