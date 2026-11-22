// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A server stopped in order under load leaves only clean checkpoints, so the next start raises
//! no `restored` (docs/adr/0109, D10; SCE_MESH.md sections 10.11 and 14.4).
//!
//! The client sends requests to many sessions as fast as the server takes them, and the server is
//! told to stop while they arrive (`Pool::stop`): it admits nothing more, runs what each session's
//! inbox holds, and saves every session in memory as a clean checkpoint. Every session the store
//! then holds is held as a clean checkpoint, which is what lets its restore raise nothing
//! (`Router::restore` tells `restored` only after a checkpoint that is not clean); and a server
//! started again on the store answers every key, its durable client's outbox holding what was
//! sent before the stop.

use std::path::Path;
use std::time::{Duration, Instant};

use sce_rust_mesh::checkpoint;
use sce_rust_session_store::SqliteStore;
use sce_server_suite::draw::seed_for;
use sce_server_suite::ledger::{take_data, TAKE};
use sce_server_suite::process::{Reported, ServerProcess};
use sce_server_suite::protocol::{Build, Command, Durable, Report, ServerOptions};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire::{self, Ids};

/// How many sessions the load reaches, and how many requests each is sent.
const SESSIONS: u64 = 24;
const REQUESTS_PER_SESSION: u64 = 4;

/// How long the server may take to say it serves, to stop, and to answer.
const PATIENCE: Duration = Duration::from_secs(30);

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

fn options(scratch: &Scratch) -> ServerOptions {
    ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        // Longer than the run: every session stays in memory until the stop saves it.
        idle_ms: 600_000,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        durable: Some(Durable {
            mailbox: 4_096,
            retain_ms: 600_000,
        }),
        build: Build::Ledger,
    }
}

/// The next report of `server` that `pick` takes, waiting at most [`PATIENCE`] for it.
fn await_report<T>(server: &ServerProcess, mut pick: impl FnMut(Report) -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        assert!(!left.is_zero(), "the server did not report in time");
        if let Some(Reported { report, .. }) = server.report_within(left) {
            if let Some(taken) = pick(report) {
                return taken;
            }
        }
    }
}

/// docs/adr/0109, D10: a stop under load saves every session it holds as a clean checkpoint,
/// none is left unsaved, and a server started on the store after it answers every key. Red if
/// `Pool::stop` takes its checkpoints as ones that are not clean, or leaves a session in memory
/// unsaved.
#[test]
fn a_server_stopped_under_load_leaves_only_clean_checkpoints() {
    let seed = seed_for("a_server_stopped_under_load_leaves_only_clean_checkpoints");
    let scratch = Scratch::new("stopped-under-load");
    let mut ids = Ids::new(seed);
    let keys: Vec<String> = (0..SESSIONS).map(|n| format!("t{n}")).collect();

    let first = ServerProcess::start(server(), &options(&scratch)).expect("a server");
    first.await_ready(PATIENCE).expect("the server serves");
    for request in 0..REQUESTS_PER_SESSION {
        for key in &keys {
            let tag = format!("{key}.r{request}");
            let asked = wire::request(&mut ids, TAKE, &take_data(key, &tag, &[]));
            first.send(&Command::Envelope(asked.bytes));
        }
    }
    // Told to stop while the requests still arrive: what it took in before is run and saved,
    // and what it takes after is refused.
    first.send(&Command::Stop(20_000));
    let (saved, not_saved) = await_report(&first, |report| match report {
        Report::Stopped { saved, not_saved } => Some((saved, not_saved)),
        _ => None,
    });
    first.stop(PATIENCE);
    assert_eq!(not_saved, 0, "a session was left unsaved by the stop");
    assert!(
        saved > 0,
        "the stop saved nothing, so nothing was under load"
    );

    let store = SqliteStore::open(scratch.store()).expect("the store the server kept");
    let mut held = 0u64;
    for key in &keys {
        let Some(loaded) = checkpoint::load(&store, key).expect("a checkpoint this core reads")
        else {
            continue;
        };
        held += 1;
        assert!(
            loaded.checkpoint.clean,
            "the session {key} was left as a checkpoint that is not clean, whose restore raises \
             `restored`"
        );
    }
    assert_eq!(
        held, saved,
        "the store holds {held} sessions and the stop saved {saved}"
    );
    drop(store);

    let second = ServerProcess::start(server(), &options(&scratch)).expect("a server again");
    second
        .await_ready(PATIENCE)
        .expect("the server serves again");
    for key in &keys {
        let tag = format!("{key}.after");
        let asked = wire::request(&mut ids, TAKE, &take_data(key, &tag, &[]));
        second.send(&Command::Envelope(asked.bytes));
    }
    let mut ack = 0u64;
    let mut epoch: Option<String> = None;
    let mut answered_after = 0usize;
    let deadline = Instant::now() + PATIENCE;
    while answered_after < keys.len() {
        assert!(
            Instant::now() < deadline,
            "{answered_after} of {} keys answered after the start",
            keys.len()
        );
        second.send(&Command::Poll {
            epoch: epoch.clone(),
            ack,
        });
        let (at, envelopes) = await_report(&second, |report| match report {
            Report::Mailbox { epoch, envelopes } => Some((epoch, envelopes)),
            _ => None,
        });
        if epoch.as_deref() != Some(at.as_str()) {
            epoch = Some(at);
            ack = 0;
        }
        for (number, bytes) in envelopes {
            if number <= ack {
                continue;
            }
            ack = number;
            let Some(seen) = wire::read(bytes) else {
                panic!("the server sent bytes that are no envelope");
            };
            if seen.is_ok_reply() && seen.data.contains(".after") {
                answered_after += 1;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    second.stop(PATIENCE);
}
