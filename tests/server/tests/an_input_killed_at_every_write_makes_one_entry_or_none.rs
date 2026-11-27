// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An input makes one entry of the event journal or none, whatever write it was killed at
//! (docs/adr/0114, D1; docs/formal/EventJournal.tla, `AppendOnce` and `NothingLost`).
//!
//! A client keeps asking a ledger that says each tag it takes into the journal, while its server
//! is killed with `SIGKILL` at points of load, checkpoint and delivery and started again on the
//! same store. An entry is appended by the write of the checkpoint that holds the state that said
//! it, so once the last server stopped, the stream of each session holds exactly the tags its
//! saved state took, in the order taken: no entry without its state, no state without its
//! entries. A request the client saw answered was taken once, so its tag is in the journal once;
//! an input whose checkpoint a kill lost made no entry, and the copy the client sent again made
//! the one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use sce_rust_session_store::SqliteStore;
use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::draw::seed_for;
use sce_server_suite::journal::journal_by_key;
use sce_server_suite::ledger::{take_data, taken_by_key, MACHINE_NAME};
use sce_server_suite::protocol::{Build, ServerOptions};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::{count_from, KILLS_VARIABLE};

/// How many sessions the client asks of.
const SESSIONS: u64 = 16;

/// How many requests the client sends each session: fewer than the session's duplicate window.
const REQUESTS_PER_SESSION: u64 = 10;

/// How many times the server is killed, unless the environment says otherwise.
const KILLS: u64 = 10;

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

#[test]
fn an_input_killed_at_every_write_makes_one_entry_or_none() {
    let name = "an_input_killed_at_every_write_makes_one_entry_or_none";
    let seed = seed_for(name);
    let scratch = Scratch::new("journaled");
    let options = ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        idle_ms: 300,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        durable: None,
        build: Build::Journaled,
    };
    let mut campaign = Campaign::new(server(), options, seed);
    let mut order: Vec<(u64, u64)> = (0..SESSIONS)
        .flat_map(|session| (0..REQUESTS_PER_SESSION).map(move |request| (session, request)))
        .collect();
    for at in (1..order.len()).rev() {
        let other = usize::try_from(campaign.draw().below(at as u64 + 1)).unwrap_or(0);
        order.swap(at, other);
    }
    for (session, request) in order {
        let key = format!("k{session}");
        let tag = format!("{key}.r{request}");
        campaign.ask(&key, &tag, &take_data(&key, &tag, &[]));
    }

    let kills = count_from(KILLS_VARIABLE, KILLS);
    let plan = Plan {
        kills,
        life_ms: (200, 1_500),
        halt_reach: 40,
        pace: 2,
        retry_answered_percent: 30,
        patience: Duration::from_secs(120),
    };
    if let Err(error) = campaign.run(&plan, |campaign| campaign.all_answered()) {
        panic!("{error}; {}", campaign.summary());
    }
    let summary = campaign.summary();
    eprintln!("{name}: {summary}");
    assert_eq!(campaign.kills(), kills, "{summary}");
    assert!(
        campaign.all_answered(),
        "a request was never answered; {summary}"
    );

    let file = SqliteStore::open(scratch.store()).expect("the store the servers kept");
    let sessions = file.of_machine(MACHINE_NAME);
    let keys: BTreeSet<String> = campaign
        .asked()
        .iter()
        .map(|asked| asked.key.clone())
        .collect();
    let taken = taken_by_key(&sessions, keys.iter().cloned())
        .unwrap_or_else(|error| panic!("{error}; {summary}"));
    let (journal, _) = journal_by_key(&file).unwrap_or_else(|error| panic!("{error}; {summary}"));
    for key in &keys {
        let said = journal.get(key).cloned().unwrap_or_default();
        assert_eq!(
            said, taken[key],
            "the stream of {key} holds {said:?}, and its saved state took {:?}; {summary}",
            taken[key]
        );
    }
    assert!(
        journal.keys().all(|key| keys.contains(key)),
        "the journal holds a stream of a key never asked of: {:?}; {summary}",
        journal.keys().collect::<Vec<_>>()
    );
    let mut entries: BTreeMap<&str, usize> = BTreeMap::new();
    for tags in journal.values() {
        for tag in tags {
            *entries.entry(tag.as_str()).or_default() += 1;
        }
    }
    for asked in campaign.asked() {
        assert_eq!(
            entries.get(asked.tag.as_str()).copied().unwrap_or(0),
            1,
            "the answered request {} made {} entries; {summary}",
            asked.tag,
            entries.get(asked.tag.as_str()).copied().unwrap_or(0)
        );
    }
    eprintln!(
        "{name}: {} requests, each one entry beside the state that took it",
        campaign.asked().len()
    );
}
