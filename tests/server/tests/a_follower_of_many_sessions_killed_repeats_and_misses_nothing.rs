// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A follower of many sessions that is killed again and again repeats entries and misses none,
//! and each takes effect once (docs/adr/0114, D4; docs/formal/EventJournal.tla, `FollowedOnce`,
//! `NonePassed` and `Followed`).
//!
//! Ledgers of many keys say their tags into the journal. Then a view that follows the ledger's
//! journal, one session per key, is served by a process killed with `SIGKILL` at points of load
//! and checkpoint, before and after a checkpoint is durable, and started again on the same store:
//! its reader starts again from the mark it last kept, below what its sessions took in, so it
//! hands them again what they took. Each session keeps the position it took in with what each
//! entry did, and drops a repeat. Once the last view's mark reaches the journal's last entry, each
//! key's view holds the tags of its key's stream, in the journal's order, each once.

use std::path::Path;
use std::time::{Duration, Instant};

use sce_rust_session_store::{JournalStore, SqliteStore};
use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::draw::{seed_for, Draw};
use sce_server_suite::journal::{journal_by_key, read_view, VIEW};
use sce_server_suite::ledger::take_data;
use sce_server_suite::process::ServerProcess;
use sce_server_suite::protocol::{Build, Halt, Report, ServerOptions, Stage};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::{count_from, KILLS_VARIABLE};

/// How many sessions the ledger keeps, and so how many views follow it.
const SESSIONS: u64 = 40;

/// How many tags each ledger takes.
const REQUESTS_PER_SESSION: u64 = 5;

/// How many times the view's server is killed, unless the environment says otherwise.
const KILLS: u64 = 12;

/// A life's halt is drawn at a count from 1 to this.
const HALT_REACH: u64 = 30;

/// How long a server may take to say it serves, and the last one to follow to the end.
const READY: Duration = Duration::from_secs(30);
const PATIENCE: Duration = Duration::from_secs(120);

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

#[test]
fn a_follower_of_many_sessions_killed_repeats_and_misses_nothing() {
    let name = "a_follower_of_many_sessions_killed_repeats_and_misses_nothing";
    let seed = seed_for(name);
    let scratch = Scratch::new("followed");
    let ledger = ServerOptions {
        store: scratch.store(),
        owner: "ledger-server".to_string(),
        idle_ms: 300,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        durable: None,
        build: Build::Journaled,
    };
    let mut campaign = Campaign::new(server(), ledger.clone(), seed);
    for request in 0..REQUESTS_PER_SESSION {
        for session in 0..SESSIONS {
            let key = format!("k{session}");
            let tag = format!("{key}.r{request}");
            campaign.ask(&key, &tag, &take_data(&key, &tag, &[]));
        }
    }
    let plan = Plan {
        kills: 0,
        life_ms: (0, 0),
        halt_reach: 1,
        pace: 8,
        retry_answered_percent: 0,
        patience: PATIENCE,
    };
    if let Err(error) = campaign.run(&plan, |campaign| campaign.all_answered()) {
        panic!("{error}; {}", campaign.summary());
    }
    assert!(campaign.all_answered(), "{}", campaign.summary());
    let file = SqliteStore::open(scratch.store()).expect("the store the ledger kept");
    let (journal, last) = journal_by_key(&file).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        journal.values().map(Vec::len).sum::<usize>() as u64,
        SESSIONS * REQUESTS_PER_SESSION,
        "the ledger said each tag it took once"
    );

    let view = ServerOptions {
        owner: "view-server".to_string(),
        build: Build::View,
        ..ledger
    };
    let mut draw = Draw::new(seed);
    let stages = [Stage::Load, Stage::CheckpointBefore, Stage::CheckpointAfter];
    let kills = count_from(KILLS_VARIABLE, KILLS);
    let mut halts: Vec<Halt> = Vec::new();
    for life in 0..kills {
        let halt = Halt {
            stage: stages[usize::try_from(life % 3).unwrap_or(0)],
            at: draw.between(1, HALT_REACH),
        };
        let options = ServerOptions {
            halt: Some(halt),
            ..view.clone()
        };
        let process = ServerProcess::start(server(), &options).expect("the view's server starts");
        process
            .await_ready(READY)
            .unwrap_or_else(|error| panic!("{error}"));
        let ends = Instant::now() + Duration::from_millis(draw.between(100, 1_000));
        while Instant::now() < ends {
            let Some(reported) = process.report_within(Duration::from_millis(20)) else {
                continue;
            };
            if let Report::Halted(reached) = reported.report {
                halts.push(reached);
                break;
            }
        }
        process.kill();
    }
    assert!(
        !halts.is_empty(),
        "no life of the view reached the halt drawn for it; seed {seed}"
    );

    // The last life is not killed: it follows until its mark is the journal's last position.
    let process = ServerProcess::start(server(), &view).expect("the view's server starts");
    process
        .await_ready(READY)
        .unwrap_or_else(|error| panic!("{error}"));
    let deadline = Instant::now() + PATIENCE;
    loop {
        let mark = file.read_through(VIEW).expect("the view's mark");
        if mark >= last {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the view's mark is {mark} after {PATIENCE:?}, below the journal's {last}; seed {seed}, \
             halts {halts:?}"
        );
        while process.report_within(Duration::from_millis(50)).is_some() {}
    }
    process.stop(Duration::from_secs(30));

    let views = file.of_machine(VIEW);
    for (key, tags) in &journal {
        let (taken, position) = read_view(&views, key)
            .unwrap_or_else(|error| panic!("{error}"))
            .unwrap_or_else(|| panic!("no view of {key} was kept; seed {seed}"));
        assert_eq!(
            &taken, tags,
            "the view of {key} took {taken:?}, and its stream holds {tags:?}; seed {seed}, \
             halts {halts:?}"
        );
        assert!(
            position <= last && position > 0,
            "the view of {key} took in the journal through {position}; seed {seed}"
        );
    }
    eprintln!(
        "{name}: {} views of {} entries, the view killed {kills} times at {halts:?}, each entry \
         taken in once",
        journal.len(),
        last
    );
}
