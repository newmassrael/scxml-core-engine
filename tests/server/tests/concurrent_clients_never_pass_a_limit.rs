// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Concurrent clients against one key, and against a value sessions share under
//! `sce-host-kv-put-if`, never pass the limit the document states (docs/adr/0107, decision D10,
//! its third bullet; D3 and D6).
//!
//! The pool is served in the test's own process, on the reference store, and every client is a
//! thread of its own that hands the pool its requests and runs the session they name, as a host
//! does from as many threads as it runs sessions on.

use std::sync::Arc;
use std::thread;

use sce_rust_host_features::Invocation;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::pool::{Pool, PoolConfig};
use sce_rust_runtime::json::{self, Value};
use sce_rust_session_store::{SessionStore, SqliteStore};
use sce_server_suite::clock::WallClock;
use sce_server_suite::ledger::{
    self, key_data, read_held, record_of, shared_host, Ledgers, Rendezvous, CLAIM, CLIENT, JOIN,
    LIMIT, PIPE, SEATS, SEATS_KEY,
};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire::{self, Ids, Recording, Seen};

/// A pool of the ledger served in this process.
type LedgerPool = Pool<Ledgers, Recording, WallClock>;

/// A pool of the ledger built by `ledgers`, kept in a reference store in `scratch`, its client's
/// link up; the store, and the transport the pool sends through.
fn pool(ledgers: Ledgers, scratch: &Scratch) -> (Arc<LedgerPool>, Arc<SqliteStore>, Recording) {
    let store = Arc::new(SqliteStore::open(scratch.store()).expect("the reference store"));
    let recording = Recording::default();
    let pool = Pool::new(
        ledger::machine(PIPE, 60_000),
        PoolConfig {
            owner: "server".to_string(),
            lease_ttl_ms: 600_000,
            renew_every_ms: 300_000,
        },
        ledgers,
        Arc::clone(&store) as Arc<dyn SessionStore>,
        recording.clone(),
        WallClock::default(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    pool.peer_ready(CLIENT);
    (Arc::new(pool), store, recording)
}

/// Hand the pool `count` requests `event` for `key` from a client of its own, each run to its
/// end before the next, on a thread of its own.
fn client(
    pool: &Arc<LedgerPool>,
    nonce: u64,
    event: &'static str,
    key: String,
    count: usize,
) -> thread::JoinHandle<()> {
    let pool = Arc::clone(pool);
    thread::spawn(move || {
        let mut ids = Ids::new(nonce);
        for _ in 0..count {
            let request = wire::request(&mut ids, event, &key_data(&key));
            let key = pool
                .receive(CLIENT, request.bytes)
                .expect("the request names its session");
            pool.run(&key);
        }
    })
}

/// The member `name` of a reply's data.
fn member(reply: &Seen, name: &str) -> Value {
    json::parse(&reply.data)
        .ok()
        .and_then(|data| data.member(name).cloned())
        .unwrap_or(Value::Null)
}

/// The replies the pool sent, each an `Ok` reply to the client.
fn replies(recording: &Recording) -> Vec<Seen> {
    recording
        .take()
        .into_iter()
        .map(|(peer, reply)| {
            assert_eq!(peer, CLIENT, "{reply:?}");
            assert_eq!(reply.pattern, PatternKind::RpcReply, "{reply:?}");
            assert!(reply.is_ok_reply(), "a request was refused: {reply:?}");
            reply
        })
        .collect()
}

/// Sixteen clients ask one key's session to admit them, four times each, all at once. The
/// session admits exactly as many as the document's limit, never says it holds more, and its
/// restored state holds that many: the pool takes one key's requests one at a time, so the rule
/// one key owns needs no lock.
#[test]
fn one_key_admits_no_more_than_its_limit_however_many_clients_ask() {
    const CLIENTS: u64 = 16;
    const EACH: usize = 4;
    let scratch = Scratch::new("one-key");
    let (pool, store, recording) = pool(Ledgers::default(), &scratch);
    let clients: Vec<_> = (0..CLIENTS)
        .map(|n| client(&pool, n + 1, JOIN, "room".to_string(), EACH))
        .collect();
    for client in clients {
        client.join().expect("no client panicked");
    }

    let replies = replies(&recording);
    assert_eq!(
        replies.len(),
        CLIENTS as usize * EACH,
        "each request answered once"
    );
    let admitted = replies
        .iter()
        .filter(|reply| member(reply, "admitted") == Value::Bool(true))
        .count();
    assert_eq!(admitted, LIMIT as usize, "{replies:?}");
    for reply in &replies {
        let Value::Number(members) = member(reply, "members") else {
            panic!("a reply that names no count of members: {reply:?}");
        };
        let members: u32 = members.parse().expect("a count");
        assert!(
            members <= LIMIT,
            "the session said it holds {members}: {reply:?}"
        );
    }
    let held = read_held(&*store, "room")
        .expect("the store reads")
        .expect("the session was saved");
    assert_eq!(held.members, LIMIT);
    assert!(pool.take_errors().is_empty());
}

/// Twelve sessions, each asked by a client of its own on a thread of its own, claim seats from a
/// count every session shares, held in the reference host's store and guarded by
/// `sce-host-kv-put-if`; each session's first claim meets the others between its read and its
/// write, so that every one but one writes over a version another already moved. Exactly as many
/// seats as the document states are granted, and the count stands there. Every conditional write
/// started again under its identity, as a restored session starts it, is answered with its first
/// result and writes nothing (docs/adr/0107, D4).
#[test]
fn sessions_that_share_a_value_never_pass_its_limit() {
    const SESSIONS: usize = 12;
    const EACH: usize = 3;
    let scratch = Scratch::new("shared-value");
    let host = shared_host();
    let rendezvous = Arc::new(Rendezvous::new(SESSIONS));
    let (pool, _store, recording) = pool(Ledgers::sharing(Arc::clone(&host), rendezvous), &scratch);
    let clients: Vec<_> = (0..SESSIONS)
        .map(|n| client(&pool, n as u64 + 1, CLAIM, format!("c{n}"), EACH))
        .collect();
    for client in clients {
        client.join().expect("no client panicked");
    }

    let replies = replies(&recording);
    assert_eq!(replies.len(), SESSIONS * EACH, "each claim answered once");
    let granted = replies
        .iter()
        .filter(|reply| member(reply, "granted") == Value::Bool(true))
        .count();
    assert_eq!(granted as u64, SEATS, "{replies:?}");
    for reply in &replies {
        let Value::Number(seats) = member(reply, "seats") else {
            panic!("a reply that names no count of seats: {reply:?}");
        };
        let seats: u64 = seats.parse().expect("a count");
        assert!(
            seats <= SEATS,
            "a session read or wrote {seats} seats taken: {reply:?}"
        );
    }

    let mut host = host.lock().expect("the shared host");
    assert_eq!(
        host.store_entries().expect("the host's store"),
        vec![(SEATS_KEY.to_string(), SEATS.to_string())]
    );
    let writes: Vec<_> = host
        .take_starts()
        .into_iter()
        .filter(|start| start.operation == "sce-host-kv-put-if")
        .collect();
    let lost = writes.len() as u64 - SEATS;
    assert!(
        lost >= SESSIONS as u64 - 1,
        "only {lost} conditional writes found the count moved, so the sessions did not race; \
         {} writes",
        writes.len()
    );

    let mut written_again = 0;
    for start in &writes {
        let invocation = Invocation::new(&start.session_id, &start.id, start.start);
        let answer = record_of(host.start(&start.operation, &invocation, &start.params))
            .unwrap_or_else(|| panic!("a repeat of {invocation:?} was not answered"));
        if answer.member("written") == Some(&Value::Bool(true)) {
            written_again += 1;
        }
    }
    assert_eq!(
        written_again, SEATS,
        "the repeats did not answer as the first starts did"
    );
    assert_eq!(
        host.store_entries().expect("the host's store"),
        vec![(SEATS_KEY.to_string(), SEATS.to_string())],
        "a repeat wrote"
    );
}
