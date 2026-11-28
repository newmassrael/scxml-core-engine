// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A request that carries a position of the journal a follower follows is answered on every entry
//! at or below it, by a follower that has read none of the journal when it is asked and by one
//! killed and started again (docs/adr/0116, D5; deploy.yaml's `sessions.position_from`).
//!
//! Ledgers of many keys say their tags into the journal. Then a view that follows the ledger's
//! journal is started on the same store, before it has read any of it, and asked, for each key,
//! how many tags it holds, each request carrying the position of the key's last tag, as an answer
//! of the ledger would tell it. The pool holds each request until its reader's mark has passed
//! that position, so each answer counts every tag of its key. The view is killed at a drawn point
//! of its work in each life but the last, which is sent again what the lives before left
//! unanswered. A request whose position is above the journal's horizon, which no answer told, is
//! refused `InvalidArgument`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use sce_rust_mesh::generated::rpc_status::RpcStatus;
use sce_rust_runtime::json::{self, Value};
use sce_rust_session_store::SqliteStore;
use sce_server_suite::campaign::{Campaign, Plan};
use sce_server_suite::draw::{seed_for, Draw};
use sce_server_suite::journal::{count_data, journal_by_key, last_position_by_key, COUNT, COUNTED};
use sce_server_suite::ledger::take_data;
use sce_server_suite::process::{Reported, ServerProcess};
use sce_server_suite::protocol::{Build, Command, Halt, Report, ServerOptions, Stage};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire::{read, request, Ids, Request};
use sce_server_suite::{count_from, KILLS_VARIABLE};

/// How many sessions the ledger keeps, and so how many views are asked.
const SESSIONS: u64 = 20;

/// How many tags each ledger takes.
const REQUESTS_PER_SESSION: u64 = 3;

/// How many lives of the view are killed before the last, unless the environment says otherwise.
const KILLS: u64 = 3;

/// A life's halt is drawn at a count from 1 to this.
const HALT_REACH: u64 = 20;

/// How long a server may take to say it serves, and the last life to answer everything.
const READY: Duration = Duration::from_secs(30);
const PATIENCE: Duration = Duration::from_secs(120);

fn server() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_sce-suite-server"))
}

/// What a reply to a request of the view said: its status, and the count it answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Answer {
    status: Option<RpcStatus>,
    taken: Option<u64>,
}

/// Note in `answers` each reply of `reported` to a request of `asked`, by its wire id; answer
/// the halt it says the server reached, if it does.
fn note(
    reported: Reported,
    asked: &BTreeMap<[u8; 16], String>,
    answers: &mut BTreeMap<[u8; 16], Answer>,
) -> Option<Halt> {
    match reported.report {
        Report::Sent(bytes) => {
            let seen = read(bytes)?;
            let wire_id = seen.wire_id.filter(|wire_id| asked.contains_key(wire_id))?;
            if seen.event != COUNTED && seen.status.is_none() {
                return None;
            }
            let taken = json::parse(&seen.data)
                .ok()
                .and_then(|data| match data.member("taken") {
                    Some(Value::Number(digits)) => digits.parse().ok(),
                    _ => None,
                });
            answers.insert(
                wire_id,
                Answer {
                    status: seen.status,
                    taken,
                },
            );
            None
        }
        Report::Halted(halt) => Some(halt),
        _ => None,
    }
}

#[test]
fn a_request_to_a_follower_is_answered_on_what_it_asks_it_to_have_read() {
    let name = "a_request_to_a_follower_is_answered_on_what_it_asks_it_to_have_read";
    let seed = seed_for(name);
    let scratch = Scratch::new("positioned");
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
    let file = SqliteStore::open(scratch.store()).expect("the store the ledger kept");
    let (journal, last) = journal_by_key(&file).unwrap_or_else(|error| panic!("{error}"));
    let positions = last_position_by_key(&file).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        positions.len() as u64,
        SESSIONS,
        "the ledger said a tag of every key"
    );

    // Each key's request, carrying the position of its last tag, and one past the horizon.
    let mut ids = Ids::new(seed);
    let requests: Vec<(String, Request)> = positions
        .iter()
        .map(|(key, after)| {
            let asked = request(&mut ids, COUNT, &count_data(key, Some(*after)));
            (key.clone(), asked)
        })
        .collect();
    let past_horizon = request(&mut ids, COUNT, &count_data("k0", Some(last + 1_000)));
    let mut asked: BTreeMap<[u8; 16], String> = requests
        .iter()
        .map(|(key, asked)| (asked.wire_id, key.clone()))
        .collect();
    asked.insert(past_horizon.wire_id, "k0".to_string());

    let view = ServerOptions {
        owner: "view-server".to_string(),
        build: Build::View,
        ..ledger
    };
    let mut draw = Draw::new(seed);
    let stages = [Stage::Load, Stage::CheckpointBefore, Stage::CheckpointAfter];
    let kills = count_from(KILLS_VARIABLE, KILLS);
    let mut answers: BTreeMap<[u8; 16], Answer> = BTreeMap::new();
    let mut halts: Vec<Halt> = Vec::new();
    for life in 0..=kills {
        let last_life = life == kills;
        let options = ServerOptions {
            halt: (!last_life).then(|| Halt {
                stage: stages[usize::try_from(life % 3).unwrap_or(0)],
                at: draw.between(1, HALT_REACH),
            }),
            ..view.clone()
        };
        let process = ServerProcess::start(server(), &options).expect("the view's server starts");
        process
            .await_ready(READY)
            .unwrap_or_else(|error| panic!("{error}"));
        // What a life before left unanswered is sent again, the same bytes, as a client that
        // never saw its answer does.
        for (_, asked_of) in requests
            .iter()
            .filter(|(_, asked_of)| !answers.contains_key(&asked_of.wire_id))
        {
            process.send(&Command::Envelope(asked_of.bytes.clone()));
        }
        if last_life {
            process.send(&Command::Envelope(past_horizon.bytes.clone()));
        }
        let ends = Instant::now()
            + if last_life {
                PATIENCE
            } else {
                Duration::from_millis(draw.between(200, 1_500))
            };
        while Instant::now() < ends && answers.len() < asked.len() {
            let Some(reported) = process.report_within(Duration::from_millis(20)) else {
                continue;
            };
            if let Some(halt) = note(reported, &asked, &mut answers) {
                halts.push(halt);
                break;
            }
        }
        let left = if last_life {
            process.stop(Duration::from_secs(30))
        } else {
            process.kill()
        };
        for reported in left {
            if let Some(halt) = note(reported, &asked, &mut answers) {
                halts.push(halt);
            }
        }
    }

    assert_eq!(
        answers
            .get(&past_horizon.wire_id)
            .map(|answer| answer.status),
        Some(Some(RpcStatus::InvalidArgument)),
        "a position above the journal's horizon is refused; seed {seed}"
    );
    for (key, asked_of) in &requests {
        let tags = journal.get(key).map_or(0, Vec::len) as u64;
        let answer = answers
            .get(&asked_of.wire_id)
            .unwrap_or_else(|| panic!("no answer for {key}; seed {seed}, halts {halts:?}"));
        assert!(
            matches!(answer.status, None | Some(RpcStatus::Ok)),
            "the view of {key} answered {answer:?}; seed {seed}, halts {halts:?}"
        );
        assert_eq!(
            answer.taken,
            Some(tags),
            "the view of {key} answered before it had read the {tags} tags its request asked \
             for; seed {seed}, halts {halts:?}"
        );
    }
    eprintln!(
        "{name}: {} views asked through the journal's {} entries, the view killed {kills} times at \
         {halts:?}, each answered on every tag its request asked it to have read",
        requests.len(),
        last
    );
}
