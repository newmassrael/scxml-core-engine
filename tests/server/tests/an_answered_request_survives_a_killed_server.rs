// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Every request a client saw answered has its effect in the restored state, and every request
//! sent again receives the first reply (docs/adr/0107, decision D10, its second bullet; D4 and
//! D5).
//!
//! A client keeps asking while its server is killed with `SIGKILL` at points of load, checkpoint
//! and delivery and started again on the same store. The client sends again what it saw no answer
//! to, and some of what it saw answered, as a client that lost its answer does. A reply leaves
//! only once the checkpoint that holds the request's effect is durable (SCE_MESH.md section
//! 10.11), and a duplicate of an answered request is answered with the reply its first copy made,
//! which the checkpoint keeps (section 10.5). So every copy of a reply the client receives is the
//! first one, byte for byte, and the store, read once the last server stopped, holds every
//! answered request's effect once, where its reply said it stands.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use sce_rust_runtime::json::{self, Value};
use sce_rust_session_store::SqliteStore;
use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::draw::seed_for;
use sce_server_suite::ledger::{take_data, taken_by_key};
use sce_server_suite::protocol::{Build, ServerOptions};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::{count_from, KILLS_VARIABLE};

/// How many sessions the client asks of.
const SESSIONS: u64 = 24;

/// How many requests the client sends each session: fewer than the session's duplicate window
/// holds (`ledger::DEDUP_WINDOW`), so that every copy of one is still known for what it is.
const REQUESTS_PER_SESSION: u64 = 12;

/// How many times the server is killed, unless the environment says otherwise.
const KILLS: u64 = 8;

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

/// The position a `take` reply says its request stands at.
fn position(data: &str) -> Option<usize> {
    match json::parse(data).ok()?.member("position") {
        Some(Value::Number(digits)) => digits.parse().ok(),
        _ => None,
    }
}

#[test]
fn an_answered_request_keeps_its_effect_and_its_first_reply_across_kills() {
    let name = "an_answered_request_keeps_its_effect_and_its_first_reply_across_kills";
    let seed = seed_for(name);
    let scratch = Scratch::new("answered");
    let options = ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        idle_ms: 300,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        durable: None,
        build: Build::Ledger,
    };
    let mut campaign = Campaign::new(server(), options, seed);
    // The requests of all sessions interleaved, in an order drawn for the run.
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
        halt_reach: 60,
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

    let mut copies = 0;
    for asked in campaign.asked() {
        let Some(first) = asked.answers.first() else {
            panic!("the request {} was never answered; {summary}", asked.tag);
        };
        for copy in &asked.answers[1..] {
            assert_eq!(
                copy.bytes, first.bytes,
                "the request {} was answered {:?} and later {:?}; {summary}",
                asked.tag, first.data, copy.data
            );
        }
        copies += asked.answers.len() - 1;
    }
    assert!(
        campaign.resent_answered() > 0 && copies > 0,
        "no answered request was sent again and answered, so the first reply was never asked \
         for twice; {summary}"
    );
    assert!(
        campaign.strays().is_empty(),
        "the server sent what the client never asked for: {:?}; {summary}",
        campaign.strays()
    );

    let store = SqliteStore::open(scratch.store()).expect("the store the servers kept");
    let keys: BTreeSet<String> = campaign
        .asked()
        .iter()
        .map(|asked| asked.key.clone())
        .collect();
    let taken = taken_by_key(&store, keys).unwrap_or_else(|error| panic!("{error}; {summary}"));
    let mut asked_of: BTreeMap<&str, usize> = BTreeMap::new();
    for asked in campaign.asked() {
        *asked_of.entry(asked.key.as_str()).or_default() += 1;
        let held = &taken[&asked.key];
        let at: Vec<usize> = held
            .iter()
            .enumerate()
            .filter(|(_, tag)| **tag == asked.tag)
            .map(|(at, _)| at + 1)
            .collect();
        let said = position(&asked.answers[0].data);
        assert_eq!(
            at.len(),
            1,
            "the answered request {} is held {} times in the restored state of {}; {summary}",
            asked.tag,
            at.len(),
            asked.key
        );
        assert_eq!(
            said,
            Some(at[0]),
            "the reply to {} said it stands at {said:?}, and the restored state holds it at {}; \
             {summary}",
            asked.tag,
            at[0]
        );
    }
    for (key, count) in asked_of {
        assert_eq!(
            taken[key].len(),
            count,
            "the restored state of {key} holds {:?}, not the {count} requests asked of it; \
             {summary}",
            taken[key]
        );
    }
    eprintln!(
        "{name}: {} requests, each held once; {copies} copies of a reply received, every one the \
         first",
        campaign.asked().len()
    );
}
