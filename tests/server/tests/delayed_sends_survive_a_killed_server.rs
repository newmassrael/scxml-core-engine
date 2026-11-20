// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Many keyed sessions, each with delayed sends at spread due times, served by a process that is
//! killed with `SIGKILL` at points of load, checkpoint and delivery and started again on the same
//! store; after every restart every send is delivered exactly once, at or after its due and never
//! before (docs/adr/0107, decision D10, its first bullet; D5's timers of sessions not loaded).
//!
//! "Exactly once" is judged as the client judges it (SCE_MESH.md section 10.11): a send leaves in
//! the checkpoint that removes it from its session's saved state, so a server killed after that
//! checkpoint is durable sends the same envelope again when it restores the session, under the
//! same id, which the client's duplicate window drops. A send delivered twice is a send that
//! reached the client under two ids.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::clock::wall_ms;
use sce_server_suite::draw::seed_for;
use sce_server_suite::ledger::{reminder_tag, take_data};
use sce_server_suite::protocol::ServerOptions;
use sce_server_suite::scratch::Scratch;
use sce_server_suite::{count_from, KILLS_VARIABLE, SESSIONS_VARIABLE};

/// How many sessions a run of the default suite holds.
const SESSIONS: u64 = 1_000;

/// The sessions the decision names, for the run that is asked for by name.
const SESSIONS_AT_SCALE: u64 = 10_000;

/// How many delayed sends each session holds.
const SENDS_PER_SESSION: usize = 2;

/// How many times the server is killed, unless the environment says otherwise.
const KILLS: u64 = 6;

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

/// A run of the scenario with `sessions` sessions.
fn delayed_sends_survive_kills(name: &str, sessions: u64) {
    let seed = seed_for(name);
    let kills = count_from(KILLS_VARIABLE, KILLS);
    let scratch = Scratch::new("delayed-sends");
    let options = ServerOptions {
        store: scratch.store(),
        owner: "server".to_string(),
        // Shorter than the dues are apart, so that most sessions are saved and dropped before
        // their sends come due, and are loaded again by the store's index of dues.
        idle_ms: 1_500,
        // Longer than the run, so that no live session spends a write on renewing its lease; a
        // server started again takes its leases over at once, under the same name.
        lease_ttl_ms: 600_000,
        renew_every_ms: 300_000,
        halt: None,
        durable: None,
    };
    let mut campaign = Campaign::new(server(), options, seed);

    // The dues spread over a span that grows with the sessions, starting after the first lives
    // have begun, so that sends come due while servers are being killed and after.
    let span_ms = (sessions * 8).max(8_000);
    let first_due = wall_ms() + 1_500;
    let mut expected: BTreeMap<String, u64> = BTreeMap::new();
    for n in 0..sessions {
        let key = format!("s{n}");
        let dues: Vec<u64> = (0..SENDS_PER_SESSION)
            .map(|_| first_due + campaign.draw().below(span_ms))
            .collect();
        for (i, due) in dues.iter().enumerate() {
            expected.insert(reminder_tag(&key, i), *due);
        }
        campaign.ask(&key, &key, &take_data(&key, &key, &dues));
    }

    let plan = Plan {
        kills,
        life_ms: (400, 2_500),
        halt_reach: (sessions / 8).max(16),
        pace: usize::try_from(sessions).unwrap_or(usize::MAX),
        retry_answered_percent: 10,
        patience: Duration::from_millis(span_ms + 180_000),
    };
    let ran = campaign.run(&plan, |campaign| {
        campaign.all_answered()
            && expected
                .keys()
                .all(|tag| campaign.deliveries().contains_key(tag))
    });
    if let Err(error) = ran {
        panic!("{error}; {}", campaign.summary());
    }
    let summary = campaign.summary();
    eprintln!("{name}: {summary}");

    assert_eq!(campaign.kills(), kills, "{summary}");
    let unanswered: Vec<&str> = campaign
        .asked()
        .iter()
        .filter(|asked| asked.answers.is_empty())
        .map(|asked| asked.tag.as_str())
        .collect();
    assert!(
        unanswered.is_empty(),
        "{} requests were never answered, among them {:?}; {summary}",
        unanswered.len(),
        &unanswered[..unanswered.len().min(10)]
    );

    let mut copies = 0;
    for (tag, due) in &expected {
        let Some(deliveries) = campaign.deliveries().get(tag) else {
            panic!("the send {tag}, due at {due}, was never delivered; {summary}");
        };
        let ids: BTreeSet<[u8; 16]> = deliveries.iter().map(|d| d.seen.id).collect();
        assert_eq!(
            ids.len(),
            1,
            "the send {tag} reached the client under {} envelope ids, so its effect twice; \
             {summary}",
            ids.len()
        );
        copies += deliveries.len() - 1;
        for delivery in deliveries {
            assert_eq!(
                delivery.due_ms, *due,
                "the send {tag} was kept with another due; {summary}"
            );
            assert!(
                delivery.sent_ms >= *due,
                "the send {tag}, due at {due}, was sent at {}; {summary}",
                delivery.sent_ms
            );
            assert!(
                delivery.read_ms >= *due,
                "the send {tag}, due at {due}, reached the client at {}; {summary}",
                delivery.read_ms
            );
            assert_eq!(
                tag.split('.').next(),
                Some(delivery.key.as_str()),
                "the send {tag} came from the session {}; {summary}",
                delivery.key
            );
        }
    }
    let unexpected: Vec<&String> = campaign
        .deliveries()
        .keys()
        .filter(|tag| !expected.contains_key(*tag))
        .collect();
    assert!(
        unexpected.is_empty(),
        "sends no request scheduled were delivered: {unexpected:?}; {summary}"
    );
    eprintln!(
        "{name}: {} sends delivered once each, {copies} copies of one dropped by id",
        expected.len()
    );
}

#[test]
fn every_delayed_send_is_delivered_once_and_never_early_across_kills() {
    delayed_sends_survive_kills(
        "every_delayed_send_is_delivered_once_and_never_early_across_kills",
        count_from(SESSIONS_VARIABLE, SESSIONS),
    );
}

/// The decision's ten thousand sessions. Minutes rather than seconds, so it runs when asked for:
/// `cargo test -p sce-server-suite --test delayed_sends_survive_a_killed_server -- --ignored`.
#[test]
#[ignore = "ten thousand sessions take minutes; run it by name with --ignored"]
fn ten_thousand_sessions_keep_their_delayed_sends_across_kills() {
    delayed_sends_survive_kills(
        "ten_thousand_sessions_keep_their_delayed_sends_across_kills",
        SESSIONS_AT_SCALE,
    );
}
