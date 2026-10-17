// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// The relay reads what strangers send it: requests, and the records a store kept. The responses a
// courier reads are what a relay it does not trust sends. Each is given what it reads when it is true
// and what a seeded mutator makes of it, and may refuse and may not panic; what it accepts it writes
// back as it read it. A relay is also asked, request by request, for whatever the mutator makes of
// every kind of request, with a token that is good: it answers each with a refusal or an answer, and
// is still the relay it was.

mod common;

use common::fuzz;
use sce_rust_relay::{
    Authority, FileStore, Limits, MemoryStore, Record, Relay, Request, Response, Store,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::Identity;

const LIST: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn requests() -> Vec<Request> {
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

#[test]
fn requests_and_responses_are_read_without_a_panic_and_written_back_as_read() {
    let items: Vec<Vec<u8>> = requests()
        .iter()
        .map(|request| request.to_json().into_bytes())
        .collect();
    fuzz("requests", &items, 6000, |input| {
        if let Ok(request) = Request::from_json(&sce_rust_assurance::lossy(input)) {
            assert_eq!(Request::from_json(&request.to_json()), Ok(request));
        }
    });

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
    fuzz("responses", &items, 6000, |input| {
        if let Ok(response) = Response::from_json(&sce_rust_assurance::lossy(input)) {
            assert_eq!(Response::from_json(&response.to_json()), Ok(response));
        }
    });
}

#[test]
fn a_relay_is_asked_for_whatever_the_mutator_makes_of_every_request_and_is_still_a_relay() {
    let authority = Authority::new([7; 32], "relay");
    // An allowance that the mutator does not spend, so that its requests reach the checks.
    let limits = Limits {
        burst: 1_000_000,
        refill_per_second: 1_000_000,
        ..Limits::default()
    };
    let mut relay = Relay::open(MemoryStore::new(), authority.clone(), limits).unwrap();
    // The relay holds a list, so that requests reach the checks behind the parser.
    let token = authority.issue(LIST, 1, u64::MAX);
    for entry in entries() {
        let text = relay.handle(
            &token,
            &Request::Append {
                entry: entry.to_json(),
            }
            .to_json(),
            0,
        );
        assert!(matches!(
            Response::from_json(&text).unwrap(),
            Response::Appended { .. }
        ));
    }
    let items: Vec<Vec<u8>> = requests()
        .iter()
        .map(|request| request.to_json().into_bytes())
        .chain(entries().iter().map(|entry| {
            Request::Append {
                entry: entry.to_json(),
            }
            .to_json()
            .into_bytes()
        }))
        .collect();
    let mut now = 0u64;
    fuzz("relay", &items, 6000, |input| {
        now += 10;
        let answer = relay.handle(&token, &sce_rust_assurance::lossy(input), now);
        // Whatever it says is a response of the protocol.
        assert!(Response::from_json(&answer).is_ok(), "{answer}");
    });
    // It is still the relay it was: the list it holds is whole, and asks nothing odd.
    let text = relay.handle(&token, &Request::Heads.to_json(), u64::MAX / 2);
    assert!(
        matches!(Response::from_json(&text).unwrap(), Response::Heads { .. }),
        "{text}"
    );
}

#[test]
fn a_store_that_was_damaged_is_read_or_refused_and_never_panics_a_relay() {
    let dir = std::env::temp_dir().join(format!(
        "sce-assurance-store-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    {
        let mut store = FileStore::open(&dir).unwrap();
        for entry in entries() {
            store.append(LIST, &Record::Entry(entry.to_json())).unwrap();
        }
    }
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let valid = std::fs::read(&file).unwrap();
    let authority = Authority::new([7; 32], "relay");
    fuzz("store-file", &[valid], 400, |input| {
        std::fs::write(&file, input).unwrap();
        if let Ok(store) = FileStore::open(&dir) {
            let _ = Relay::open(store, authority.clone(), Limits::default());
        }
        // What is read is what a store reads: each line a record, or the whole refused.
        if let Ok(rows) = FileStore::open(&dir).and_then(|mut store| store.load()) {
            for (_, record) in rows {
                assert_eq!(Record::from_value(&record.to_value()), Some(record));
            }
        }
    });
    std::fs::remove_dir_all(&dir).unwrap();
}
