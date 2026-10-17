// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The relay reads what strangers send it: requests, and the records a store kept. The responses a
//! courier reads are what a relay it does not trust sends. A relay is also asked, request by request,
//! for whatever a mutator makes of every kind of request, with a token that is good: it answers each
//! with a refusal or an answer, and is still the relay it was.

use std::path::PathBuf;

use sce_rust_relay::{
    Authority, FileStore, Limits, MemoryStore, Record, Relay, Request, Response, Store,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;

use super::{text, Target};

const LIST: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn some_requests() -> Vec<Request> {
    vec![
        Request::Entries { from: 3 },
        Request::Append {
            entry: "{\"x\":1}".to_string(),
        },
        Request::Submit {
            event: "todo.write".to_string(),
            data: "{\"id\":\"1\"}".to_string(),
        },
        Request::Checkpoint {
            checkpoint: "{}".to_string(),
        },
        Request::PublishKeyPackage {
            data: "AAAA".to_string(),
        },
        Request::PublishCommit {
            epoch: 2,
            data: "BBBB".to_string(),
        },
        Request::PublishWelcome {
            epoch: 2,
            node: 3,
            data: "CCCC".to_string(),
        },
        Request::Feed {
            cursor: 0,
            limit: 50,
        },
        Request::Operations {
            node: 1,
            from: 1,
            to: 9,
        },
        Request::Heads,
    ]
}

fn entries() -> Vec<Entry> {
    let genesis = Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin())
    .finish();
    let scratch = sce_rust_trust::Trust::open(LIST);
    scratch.append(&genesis).unwrap();
    let mut all = vec![genesis];
    for node in [1u64, 2] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role: 2,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

pub fn requests() -> Target {
    let items: Vec<Vec<u8>> = some_requests()
        .iter()
        .map(|request| request.to_json().into_bytes())
        .collect();
    Target::new("requests", 6000, items, |input| {
        if let Ok(request) = Request::from_json(&text(input)) {
            assert_eq!(Request::from_json(&request.to_json()), Ok(request));
        }
    })
}

pub fn responses() -> Target {
    let responses = [
        Response::Entries {
            entries: vec!["a".to_string()],
            len: 3,
        },
        Response::Appended { new: true, len: 4 },
        Response::Accepted {
            new: false,
            cursor: 9,
        },
        Response::Feed {
            records: vec![
                Record::Entry("e".to_string()),
                Record::Operation {
                    event: "todo.write".to_string(),
                    data: "d".to_string(),
                },
                Record::KeyPackage {
                    node: 3,
                    data: "k".to_string(),
                },
                Record::Commit {
                    epoch: 2,
                    data: "m".to_string(),
                },
                Record::Welcome {
                    epoch: 2,
                    node: 3,
                    data: "w".to_string(),
                },
            ],
            next: 5,
            more: false,
        },
        Response::Operations {
            operations: vec![("todo.write".to_string(), "d".to_string())],
        },
        Response::refused(
            sce_rust_relay::Code::RateLimited { retry_after_ms: 5 },
            "slow",
        ),
    ];
    let items: Vec<Vec<u8>> = responses
        .iter()
        .map(|response| response.to_json().into_bytes())
        .collect();
    Target::new("responses", 6000, items, |input| {
        if let Ok(response) = Response::from_json(&text(input)) {
            assert_eq!(Response::from_json(&response.to_json()), Ok(response));
        }
    })
}

/// A relay that holds a list, so that a request reaches the checks behind the parser, with an
/// allowance a request does not spend.
pub fn relay() -> Target {
    let authority = Authority::new([7; 32], "relay");
    let token = authority.issue(LIST, 1, u64::MAX);
    let held = entries();
    let items: Vec<Vec<u8>> = some_requests()
        .iter()
        .map(|request| request.to_json().into_bytes())
        .chain(held.iter().map(|entry| {
            Request::Append {
                entry: entry.to_json(),
            }
            .to_json()
            .into_bytes()
        }))
        .collect();
    Target::new("relay", 2500, items, move |input| {
        let limits = Limits {
            burst: 1_000_000,
            refill_per_second: 1_000_000,
            ..Limits::default()
        };
        let mut relay = Relay::open(MemoryStore::new(), authority.clone(), limits).unwrap();
        for entry in &held {
            let answer = relay.handle(
                &token,
                &Request::Append {
                    entry: entry.to_json(),
                }
                .to_json(),
                0,
            );
            assert!(matches!(
                Response::from_json(&answer).unwrap(),
                Response::Appended { .. }
            ));
        }
        let answer = relay.handle(&token, &text(input), 10);
        // Whatever it says is a response of the protocol.
        assert!(Response::from_json(&answer).is_ok(), "{answer}");
        // It is still the relay it was: the list it holds is whole, and it answers what it is asked.
        let answer = relay.handle(&token, &Request::Heads.to_json(), 20);
        assert!(
            matches!(
                Response::from_json(&answer).unwrap(),
                Response::Heads { .. }
            ),
            "{answer}"
        );
    })
}

/// A directory made for one run, removed when the target goes.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A store that was damaged is read or refused, and never panics a relay.
pub fn store_file() -> Target {
    let dir = Scratch(std::env::temp_dir().join(format!(
        "sce-assurance-store-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    )));
    {
        let mut store = FileStore::open(&dir.0).unwrap();
        for entry in entries() {
            store.append(LIST, &Record::Entry(entry.to_json())).unwrap();
        }
    }
    let file = std::fs::read_dir(&dir.0)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let valid = std::fs::read(&file).unwrap();
    let authority = Authority::new([7; 32], "relay");
    Target::new("store-file", 400, vec![valid], move |input| {
        std::fs::write(&file, input).unwrap();
        if let Ok(store) = FileStore::open(&dir.0) {
            let _ = Relay::open(store, authority.clone(), Limits::default());
        }
        // What is read is what a store reads: each line a record, or the whole refused.
        if let Ok(rows) = FileStore::open(&dir.0).and_then(|mut store| store.load()) {
            for (_, record) in rows {
                assert_eq!(Record::from_value(&record.to_value()), Some(record));
            }
        }
    })
}
