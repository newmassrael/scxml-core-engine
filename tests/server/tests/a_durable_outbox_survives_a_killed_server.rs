// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a server sends a durable client is kept until the client acknowledges it, across kills
//! (docs/adr/0109, D10; the suite of docs/adr/0107 D10, extended).
//!
//! The client's binding declares `receipt: durable`: every reply a session makes is appended to
//! the client's outbox in the store by the same transaction that writes the session's checkpoint,
//! and the client reads it by polls, each acknowledging what it took in, which deletes it in the
//! store. The server is killed with `SIGKILL` at points of that commit, of an acknowledgement and
//! of an expiry, and started again on the same store, while the client keeps asking, goes away
//! for a while now and then, and now and then loses a poll's response before it takes it in. So
//! every reply the client was sent is delivered at least once, which is the outbox kept across
//! the kills, and taken in once, which is its numbering kept with it; none is lost, and the store
//! holds every answered request's effect once.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use sce_rust_session_store::{SessionStore, SqliteStore};
use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::draw::seed_for;
use sce_server_suite::ledger::{take_data, taken_by_key, CLIENT};
use sce_server_suite::protocol::{Durable, ServerOptions, Stage};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::{count_from, KILLS_VARIABLE};

/// How many sessions the client asks of.
const SESSIONS: u64 = 16;

/// How many requests the client sends each session: fewer than the session's duplicate window
/// holds, so that every copy of one is known for what it is.
const REQUESTS_PER_SESSION: u64 = 8;

/// How many times the server is killed, unless the environment says otherwise: one life for
/// each stage of the durable binding's work, and one or more on the clock.
const KILLS: u64 = 9;

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

#[test]
fn every_reply_a_durable_client_was_sent_is_taken_in_once_across_kills() {
    let name = "every_reply_a_durable_client_was_sent_is_taken_in_once_across_kills";
    let seed = seed_for(name);
    let scratch = Scratch::new("durable-outbox");
    let options = ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        idle_ms: 300,
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        // A window no run fills, and a retain no absence reaches: every reply is kept until the
        // client acknowledges it.
        durable: Some(Durable {
            mailbox: 4_096,
            retain_ms: 600_000,
        }),
    };
    let mut campaign = Campaign::new(server(), options, seed);
    campaign.durable(3, (150, 900), 15);
    let mut order: Vec<(u64, u64)> = (0..SESSIONS)
        .flat_map(|session| (0..REQUESTS_PER_SESSION).map(move |request| (session, request)))
        .collect();
    for at in (1..order.len()).rev() {
        let other = usize::try_from(campaign.draw().below(at as u64 + 1)).unwrap_or(0);
        order.swap(at, other);
    }
    for (session, request) in order {
        let key = format!("d{session}");
        let tag = format!("{key}.r{request}");
        campaign.ask(&key, &tag, &take_data(&key, &tag, &[]));
    }

    let kills = count_from(KILLS_VARIABLE, KILLS);
    let plan = Plan {
        kills,
        life_ms: (300, 1_500),
        halt_reach: 30,
        pace: 2,
        retry_answered_percent: 30,
        patience: Duration::from_secs(120),
    };
    if let Err(error) = campaign.run(&plan, |campaign| campaign.all_answered()) {
        panic!("{error}; {}", campaign.summary());
    }
    let summary = campaign.summary();
    let (lost_responses, absences, incarnations) = campaign.durable_counts();
    eprintln!(
        "{name}: {summary}; {lost_responses} responses lost, {absences} absences, \
         {incarnations} incarnations"
    );
    assert_eq!(campaign.kills(), kills, "{summary}");
    assert!(
        campaign
            .halts()
            .iter()
            .all(|halt| Stage::DURABLE.contains(&halt.stage)),
        "a halt of a stage a durable server does not reach: {summary}"
    );
    assert!(
        lost_responses > 0 && absences > 0,
        "the client never lost a response or went away, so nothing was asked of the outbox \
         that a client which never misses an exchange would not show; {summary}"
    );
    assert_eq!(
        incarnations, 1,
        "the outbox numbered anew, so its incarnation did not outlive a kill; {summary}"
    );
    for asked in campaign.asked() {
        assert_eq!(
            asked.answers.len(),
            1,
            "the request {} was taken in {} times; {summary}",
            asked.tag,
            asked.answers.len()
        );
    }
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
    for asked in campaign.asked() {
        let held = taken[&asked.key]
            .iter()
            .filter(|tag| **tag == asked.tag)
            .count();
        assert_eq!(
            held, 1,
            "the answered request {} is held {held} times in the restored state of {}; \
             {summary}",
            asked.tag, asked.key
        );
    }
    let outbox = store
        .outbox(CLIENT)
        .expect("the outbox read")
        .expect("the client's outbox, opened by its first poll");
    let asked = u64::try_from(campaign.asked().len()).unwrap_or(u64::MAX);
    assert!(
        outbox.next > asked,
        "the outbox numbers {} next, so fewer than the {asked} replies were appended to it in \
         one numbering across the kills; {summary}",
        outbox.next
    );
}
