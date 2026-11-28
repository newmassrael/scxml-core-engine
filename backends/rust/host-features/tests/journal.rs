// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A page of the event journal through the reference host (docs/SCE_HOST_FEATURES.md,
// section 16; docs/adr/0114, D5): the entries of one stream in the order of when
// each holds from and then of position, a page at a time from a cursor, as of the
// position the first page names; the page's deterministic CBOR, which the
// runtime's page description reads back; and the requests the host refuses.

use std::sync::Arc;

use sce_rust_host_features::{
    HttpRequest, HttpResponse, Invocation, JournalStore, MemoryStore, Outcome, Params,
    ReferenceHost, ScriptedClock, Transport, TransportFailure,
};
use sce_rust_session_store::{Fact, FactData, Journal, SessionStore, Write};
use serde_json::Value;

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

const READ: &str = "sce-host-journal-read";

/// A journal of three facts of the observed stream `directory`: one recorded at
/// 1000 that holds from 30, one at 2000 that holds from 10, and one at 3000 that
/// holds from its recorded moment.
fn journal() -> Arc<MemoryStore> {
    let store = Arc::new(MemoryStore::new());
    for (origin, valid_ms, data, now_ms) in [
        ("a", Some(30), b"one".as_slice(), 1_000),
        ("b", Some(10), b"two".as_slice(), 2_000),
        ("c", None, b"three".as_slice(), 3_000),
    ] {
        store
            .observe(
                "directory",
                origin,
                &Fact {
                    schema: "member.joined",
                    revision: 1,
                    data: FactData::Bytes(data),
                    valid_ms,
                    actor: None,
                    read: &[],
                },
                now_ms,
            )
            .expect("an observation");
    }
    store
}

fn host(store: Arc<MemoryStore>) -> ReferenceHost<Unreachable> {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    host.set_journal_store(store as Arc<dyn JournalStore>);
    host
}

fn params(pairs: &[(&str, &str)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// A read of `directory` as of `as_of`, after the cursor `after`, of `limit`.
fn read(as_of: &str, after: (&str, &str), limit: &str) -> Params {
    params(&[
        ("stream", "directory"),
        ("observed", "true"),
        ("key", ""),
        ("as_of_position", as_of),
        ("valid_ms", "9223372036854775807"),
        ("after_valid_ms", after.0),
        ("after_position", after.1),
        ("limit", limit),
    ])
}

fn start(host: &mut ReferenceHost<Unreachable>, params: &Params) -> Outcome {
    host.start(READ, &Invocation::new("session", "page", 1), params)
        .expect("a standard type")
}

/// The answer's fields, and the page's entries as the runtime reads them.
fn page(outcome: Outcome) -> (Value, Vec<Value>) {
    let Outcome::Done(data) = outcome else {
        panic!("the host did not complete: {outcome:?}");
    };
    let answer: Value = serde_json::from_str(&data).expect("a result is JSON");
    let text = answer["entries"].as_str().expect("the page's text");
    let bytes: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(u32::from(c)).expect("a byte"))
        .collect();
    let decoded = sce_rust_runtime::record_cbor::decode(
        &bytes,
        &sce_rust_runtime::journal::page_description(64, 65536),
    )
    .expect("a page of the description");
    let entries: Vec<Value> = serde_json::from_str(&decoded).expect("JSON");
    (answer, entries)
}

/// docs/adr/0114, D5: a reading is pages of one moment, ordered by when each fact
/// holds from and then by position, each resumed after the last entry of the one
/// before. Red if the host reads by position, or the next page moves its moment.
#[test]
fn a_reading_is_pages_of_one_moment_in_the_order_facts_hold() {
    let store = journal();
    let mut host = host(Arc::clone(&store));
    let (first, entries) = page(start(&mut host, &read("0", ("0", "0"), "2")));
    assert_eq!(first["error"], "");
    let as_of = first["as_of_position"]
        .as_str()
        .expect("a position")
        .to_string();
    assert_eq!(as_of, first["horizon"].as_str().expect("the horizon"));
    let holds: Vec<&str> = entries
        .iter()
        .map(|e| e["holds_ms"].as_str().unwrap())
        .collect();
    assert_eq!(holds, ["10", "30"]);
    assert_eq!(entries[0]["origin"], "b");
    assert_eq!(entries[0]["schema"], "member.joined");
    let last = &entries[1];
    // A fact observed after the first page is of a later moment, and no page of
    // this reading holds it.
    store
        .observe(
            "directory",
            "d",
            &Fact {
                schema: "member.joined",
                revision: 1,
                data: FactData::Bytes(b"four"),
                valid_ms: Some(0),
                actor: None,
                read: &[],
            },
            4_000,
        )
        .expect("an observation");
    let (second, rest) = page(start(
        &mut host,
        &read(
            &as_of,
            (
                last["holds_ms"].as_str().unwrap(),
                last["position"].as_str().unwrap(),
            ),
            "2",
        ),
    ));
    assert_eq!(second["as_of_position"], as_of.as_str());
    assert_eq!(rest.len(), 1, "{rest:?}");
    assert_eq!(rest[0]["origin"], "c");
}

/// The host refuses what is no read: a key on an observed stream, a limit of 0 or
/// past `journal_page`, a number that is none, and a position above the horizon.
#[test]
fn a_request_that_is_no_read_is_refused() {
    let mut host = host(journal());
    let refused = |outcome: Outcome| matches!(outcome, Outcome::Refuse(_));
    let mut keyed = read("0", ("0", "0"), "1");
    keyed.insert("key".to_string(), "k".to_string());
    assert!(refused(start(&mut host, &keyed)));
    assert!(refused(start(&mut host, &read("0", ("0", "0"), "0"))));
    assert!(refused(start(&mut host, &read("0", ("0", "0"), "65"))));
    assert!(refused(start(&mut host, &read("x", ("0", "0"), "1"))));
    assert!(refused(start(&mut host, &read("99", ("0", "0"), "1"))));
}

/// docs/adr/0114, D4: a machine's stream is read under the empty key, the stream of
/// the one session of a pool that keeps one (`sessions: { key_from: none }`),
/// whose key no envelope gives. Red if the host refuses a machine's stream for its
/// empty key.
#[test]
fn the_stream_of_a_pools_one_session_is_read_under_the_empty_key() {
    let store = Arc::new(MemoryStore::new());
    let ledger = store.of_machine("ledger");
    let lease = ledger
        .acquire("", "p1", 10_000, 1_000)
        .expect("the one session's lease");
    let facts = [Fact {
        schema: "ledger.kept",
        revision: 1,
        data: FactData::Bytes(b"kept"),
        valid_ms: None,
        actor: None,
        read: &[],
    }];
    ledger
        .write(
            "",
            &Write {
                journal: Some(Journal {
                    machine: "ledger",
                    facts: &facts,
                }),
                now_ms: 1_000,
                ..Write::put(b"state", None, 0, lease.fence)
            },
        )
        .expect("the one session's fact");
    let mut host = host(store);
    let mut session = read("0", ("0", "0"), "1");
    session.insert("stream".to_string(), "ledger".to_string());
    session.insert("observed".to_string(), "false".to_string());
    let (answer, entries) = page(start(&mut host, &session));
    assert_eq!(answer["error"], "");
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["schema"], "ledger.kept");
}
