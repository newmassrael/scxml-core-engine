// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A relay between the devices of a list whose members do not trust the host between them. It holds
// the membership log, the operations of every device's chain and the members' checkpoints, and it
// holds no key. What is asked of it here: that it keeps what verifies and refuses what does not (a
// stranger, a reader that writes, a device taken off past its last number, a change to what a
// member signed, plaintext in a list that keeps its content sealed); that a token names a device of
// one list and nothing else; that a device's allowance is its own; that what it keeps comes back
// after a restart through the same checks, and a store that was tampered with does not start it;
// and that a host that is given what it holds takes it in as it would from anyone.

use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_relay::{
    Authority, Code, Courier, DirectLink, FileStore, Limits, Link, LinkError, MemoryStore, Record,
    Relay, Request, Response, Store,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link as OperationLink};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ADMIN: u8 = 9;

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([ADMIN; 32])
}

/// The entries of the list: alice (1) and bob (2) write, carol (3) reads; dave (4) is no member.
fn entries() -> Vec<Entry> {
    let scratch = Trust::open(LIST);
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
    scratch.append(&genesis).unwrap();
    let mut all = vec![genesis];
    for (node, role) in [(1u64, 2u8), (2, 2), (3, 1)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
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

fn host(own: Option<u64>) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries() {
        trust.append(&entry).unwrap();
    }
    if let Some(node) = own {
        trust.set_identity(node, person(node));
    }
    trust
}

fn authority() -> Authority {
    Authority::new([7; 32], "relay-a")
}

fn relay_over<S: Store>(store: S, limits: Limits) -> Relay<S> {
    Relay::open(store, authority(), limits).unwrap()
}

fn relay() -> Relay<MemoryStore> {
    relay_over(MemoryStore::new(), Limits::default())
}

fn token(node: u64) -> String {
    authority().issue(LIST, node, 1_000_000)
}

fn ask<S: Store>(relay: &mut Relay<S>, node: u64, request: Request, now: u64) -> Response {
    let text = relay.handle(&token(node), &request.to_json(), now);
    Response::from_json(&text).unwrap()
}

/// The relay is given the whole log by `node`, in order.
fn give_log<S: Store>(relay: &mut Relay<S>, node: u64) {
    for entry in entries() {
        let response = ask(
            relay,
            node,
            Request::Append {
                entry: entry.to_json(),
            },
            0,
        );
        assert!(
            matches!(response, Response::Appended { new: true, .. }),
            "{response:?}"
        );
    }
}

fn write(author: u64, id: u64, wall: i64) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "1", "text": "",
           "wallTime": wall.to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

/// What a person's host sends of a write: the operation as it travels.
fn operation(host: &mut Trust, author: u64, id: u64, wall: i64) -> (String, String) {
    let sent = host
        .seal("#server", "todo.write", &write(author, id, wall))
        .unwrap()
        .unwrap();
    ("todo.write".to_string(), sent)
}

fn submit<S: Store>(relay: &mut Relay<S>, node: u64, op: &(String, String)) -> Response {
    ask(
        relay,
        node,
        Request::Submit {
            event: op.0.clone(),
            data: op.1.clone(),
        },
        0,
    )
}

fn refused(response: &Response) -> &Code {
    match response {
        Response::Refused { code, .. } => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_token_names_a_device_of_one_list_and_nothing_else_is_answered() {
    let mut relay = relay();
    let expired = authority().issue(LIST, 1, 5);
    for bad in ["", "nonsense", expired.as_str()] {
        let text = relay.handle(bad, &Request::Heads.to_json(), 10);
        let response = Response::from_json(&text).unwrap();
        assert_eq!(refused(&response), &Code::Unauthorized, "{bad}");
    }
    let other_relay = Authority::new([7; 32], "relay-b").issue(LIST, 1, 1_000_000);
    let text = relay.handle(&other_relay, &Request::Heads.to_json(), 0);
    assert_eq!(
        refused(&Response::from_json(&text).unwrap()),
        &Code::Unauthorized
    );

    // A token for another list reaches that list and not this one.
    let work = authority().issue("work", 1, 1_000_000);
    give_log(&mut relay, 1);
    let text = relay.handle(&work, &Request::Heads.to_json(), 0);
    assert_eq!(
        refused(&Response::from_json(&text).unwrap()),
        &Code::Forbidden,
        "the relay holds no list `work`"
    );
}

#[test]
fn a_relay_takes_the_log_in_order_and_gives_it_back_by_range() {
    let mut relay = relay();
    let all = entries();
    // A position the relay has not reached waits for the ones before it.
    let response = ask(
        &mut relay,
        1,
        Request::Append {
            entry: all[1].to_json(),
        },
        0,
    );
    assert!(
        matches!(refused(&response), Code::Invalid | Code::Behind),
        "{response:?}"
    );

    give_log(&mut relay, 1);
    // The same entry again is held once.
    let again = ask(
        &mut relay,
        1,
        Request::Append {
            entry: all[2].to_json(),
        },
        0,
    );
    assert_eq!(again, Response::Appended { new: false, len: 4 });
    assert_eq!(relay.records(LIST), 4);

    let Response::Entries { entries: got, len } =
        ask(&mut relay, 4, Request::Entries { from: 2 }, 0)
    else {
        panic!("entries");
    };
    assert_eq!(len, 4);
    assert_eq!(got, vec![all[2].to_json(), all[3].to_json()]);

    // An entry no administrator signed is refused.
    let forged = host(None)
        .draft(Body::Grant {
            node: 4,
            role: 2,
            key: person(4).public(),
            final_seq: None,
        })
        .sign(&person(4))
        .finish();
    let response = ask(
        &mut relay,
        4,
        Request::Append {
            entry: forged.to_json(),
        },
        0,
    );
    assert_eq!(refused(&response), &Code::Invalid);
}

#[test]
fn an_operation_that_verifies_is_kept_once_and_one_that_does_not_is_refused() {
    let mut relay = relay();
    give_log(&mut relay, 1);
    let mut alice = host(Some(1));
    let op = operation(&mut alice, 1, 1, 1000);

    let first = submit(&mut relay, 1, &op);
    assert_eq!(
        first,
        Response::Accepted {
            new: true,
            cursor: 5
        }
    );
    let again = submit(&mut relay, 2, &op);
    assert_eq!(
        again,
        Response::Accepted {
            new: false,
            cursor: 5
        },
        "another device hands the same operation on: held once"
    );
    assert_eq!(relay.records(LIST), 5);

    // A change to what alice signed is no operation of hers.
    let mut changed: Value = serde_json::from_str(&op.1).unwrap();
    changed["value"] = json!("2");
    let response = submit(&mut relay, 1, &(op.0.clone(), changed.to_string()));
    assert_eq!(refused(&response), &Code::Invalid);

    // Not a signed event, not JSON.
    for (event, data) in [("todo.edit", op.1.as_str()), ("todo.write", "{")] {
        let response = submit(&mut relay, 1, &(event.to_string(), data.to_string()));
        assert_eq!(refused(&response), &Code::Invalid, "{event} {data}");
    }
    assert_eq!(relay.records(LIST), 5, "none of them was kept");
}

#[test]
fn a_reader_reads_and_does_not_write_and_a_stranger_does_neither() {
    let mut relay = relay();
    give_log(&mut relay, 1);
    let mut alice = host(Some(1));
    let op = operation(&mut alice, 1, 1, 1000);
    submit(&mut relay, 1, &op);

    assert_eq!(
        refused(&submit(&mut relay, 3, &op)),
        &Code::Forbidden,
        "carol reads"
    );
    assert!(matches!(
        ask(
            &mut relay,
            3,
            Request::Feed {
                cursor: 0,
                limit: 10
            },
            0
        ),
        Response::Feed { .. }
    ));
    for request in [
        Request::Feed {
            cursor: 0,
            limit: 10,
        },
        Request::Heads,
        Request::Operations {
            node: 1,
            from: 1,
            to: 1,
        },
    ] {
        let response = ask(&mut relay, 4, request, 0);
        assert_eq!(refused(&response), &Code::Forbidden, "dave is no member");
    }
    assert_eq!(
        refused(&submit(&mut relay, 4, &op)),
        &Code::Forbidden,
        "dave writes nothing"
    );
}

#[test]
fn an_operation_that_names_an_entry_the_relay_lacks_waits_for_the_log() {
    let mut relay = relay();
    // Only the genesis and alice's grant are given: alice's operation names the latest entry.
    for entry in &entries()[..2] {
        ask(
            &mut relay,
            1,
            Request::Append {
                entry: entry.to_json(),
            },
            0,
        );
    }
    let mut alice = host(Some(1));
    let op = operation(&mut alice, 1, 1, 1000);
    assert_eq!(refused(&submit(&mut relay, 1, &op)), &Code::Behind);

    for entry in &entries()[2..] {
        ask(
            &mut relay,
            1,
            Request::Append {
                entry: entry.to_json(),
            },
            0,
        );
    }
    assert!(matches!(
        submit(&mut relay, 1, &op),
        Response::Accepted { new: true, .. }
    ));
}

#[test]
fn a_device_taken_off_may_not_write_past_its_last_number_and_what_it_made_stays() {
    let mut relay = relay();
    give_log(&mut relay, 1);
    let mut bob = host(Some(2));
    let first = operation(&mut bob, 2, 1, 1000);
    let second = operation(&mut bob, 2, 2, 1001);
    submit(&mut relay, 2, &first);

    // The administrator takes bob off after his first operation.
    let scratch = host(None);
    let removal = scratch
        .draft(Body::Grant {
            node: 2,
            role: 0,
            key: person(2).public(),
            final_seq: Some(1),
        })
        .sign(&admin())
        .finish();
    let response = ask(
        &mut relay,
        1,
        Request::Append {
            entry: removal.to_json(),
        },
        0,
    );
    assert!(
        matches!(response, Response::Appended { new: true, .. }),
        "{response:?}"
    );

    assert_eq!(
        refused(&submit(&mut relay, 2, &second)),
        &Code::Forbidden,
        "bob's own session is refused"
    );
    assert_eq!(
        refused(&submit(&mut relay, 1, &second)),
        &Code::Invalid,
        "and so is his operation past the last the administrators accepted, from anyone"
    );
    let Response::Feed { records, .. } = ask(
        &mut relay,
        1,
        Request::Feed {
            cursor: 0,
            limit: 50,
        },
        0,
    ) else {
        panic!("feed");
    };
    assert!(
        records
            .iter()
            .any(|record| matches!(record, Record::Operation { data, .. } if *data == first.1)),
        "what he made before stays"
    );
}

#[test]
fn a_feed_gives_the_records_in_the_order_they_arrived_by_cursor() {
    let mut relay = relay_over(
        MemoryStore::new(),
        Limits {
            max_feed: 3,
            ..Limits::default()
        },
    );
    give_log(&mut relay, 1);
    let mut alice = host(Some(1));
    for n in 1..=4 {
        submit(&mut relay, 1, &operation(&mut alice, 1, n, 1000 + n as i64));
    }
    assert_eq!(relay.records(LIST), 8);

    let mut seen = Vec::new();
    let mut cursor = 0;
    loop {
        let Response::Feed {
            records,
            next,
            more,
        } = ask(&mut relay, 2, Request::Feed { cursor, limit: 100 }, 0)
        else {
            panic!("feed");
        };
        assert!(records.len() <= 3, "the relay's own bound holds");
        seen.extend(records);
        cursor = next;
        if !more {
            break;
        }
    }
    assert_eq!(seen.len(), 8);
    assert!(matches!(seen[0], Record::Entry(_)));
    assert!(matches!(seen[4], Record::Operation { .. }));

    let response = ask(
        &mut relay,
        2,
        Request::Feed {
            cursor: 99,
            limit: 1,
        },
        0,
    );
    assert_eq!(refused(&response), &Code::Invalid, "a cursor past the end");

    let Response::Operations { operations } = ask(
        &mut relay,
        2,
        Request::Operations {
            node: 1,
            from: 2,
            to: 3,
        },
        0,
    ) else {
        panic!("operations");
    };
    assert_eq!(operations.len(), 2);
    let Response::Heads {
        heads,
        log_len,
        cursor,
    } = ask(&mut relay, 2, Request::Heads, 0)
    else {
        panic!("heads");
    };
    assert_eq!((heads.len(), heads[0].node, heads[0].seq), (1, 1, 4));
    assert_eq!((log_len, cursor), (4, 8));
}

#[test]
fn a_device_that_spends_its_allowance_waits_and_the_others_do_not() {
    let limits = Limits {
        burst: 4,
        refill_per_second: 1,
        write_cost: 1,
        ..Limits::default()
    };
    let mut relay = relay_over(MemoryStore::new(), limits);
    give_log(&mut relay, 1);
    // The log took the four requests of alice's allowance; she is spent, bob is not.
    let spent = ask(&mut relay, 1, Request::Heads, 0);
    let Response::Refused {
        code: Code::RateLimited { retry_after_ms },
        ..
    } = spent
    else {
        panic!("{spent:?}");
    };
    assert!(retry_after_ms > 0);
    assert!(matches!(
        ask(&mut relay, 2, Request::Heads, 0),
        Response::Heads { .. }
    ));
    assert!(matches!(
        ask(&mut relay, 1, Request::Heads, retry_after_ms),
        Response::Heads { .. }
    ));
}

#[test]
fn a_request_larger_than_the_relay_takes_is_refused_before_it_is_read() {
    let mut relay = relay_over(
        MemoryStore::new(),
        Limits {
            max_request_bytes: 64,
            ..Limits::default()
        },
    );
    let text = relay.handle(
        &token(1),
        &Request::Append {
            entry: "x".repeat(200),
        }
        .to_json(),
        0,
    );
    assert_eq!(
        refused(&Response::from_json(&text).unwrap()),
        &Code::TooLarge
    );
}

/// A list that keeps its content sealed: the relay checks that what it is given is sealed under an
/// epoch the log began, holds it as it is, and what it holds says nothing of what was written.
#[test]
fn a_relay_holds_sealed_content_of_one_size_and_refuses_plaintext_in_a_sealed_list() {
    let mut relay = relay();
    give_log(&mut relay, 1);
    let epoch = host(None)
        .draft(Body::Epoch { epoch: 1 })
        .sign(&admin())
        .finish();
    assert!(matches!(
        ask(
            &mut relay,
            1,
            Request::Append {
                entry: epoch.to_json()
            },
            0
        ),
        Response::Appended { new: true, .. }
    ));

    let mut alice = host(Some(1));
    alice.append(&epoch).unwrap();
    alice.set_epoch_key(1, [5; 32]);
    let titled = |id: u64, text: &str| {
        json!({"id": id.to_string(), "field": "3", "value": "0", "text": text,
               "wallTime": (1000 + id).to_string(), "counter": "0", "nodeId": "1"})
        .to_string()
    };
    let mut sent = Vec::new();
    let longest = "ABCDEFGHIJKLMNOPQRSTUVWXYZ012345";
    assert_eq!(longest.len(), 32);
    for (id, text) in [(1u64, "buy milk"), (2, ""), (3, longest)] {
        let data = alice
            .seal("#server", "todo.write", &titled(id, text))
            .unwrap()
            .unwrap();
        let response = submit(&mut relay, 1, &("todo.write".to_string(), data.clone()));
        assert!(
            matches!(response, Response::Accepted { new: true, .. }),
            "{response:?}"
        );
        sent.push(data);
    }
    let sizes: Vec<usize> = sent
        .iter()
        .map(|data| {
            let value: Value = serde_json::from_str(data).unwrap();
            value["text"].as_str().unwrap().len()
        })
        .collect();
    assert_eq!(
        sizes,
        [84, 84, 84],
        "a word, a number and a line are one size to the relay"
    );

    let Response::Feed { records, .. } = ask(
        &mut relay,
        2,
        Request::Feed {
            cursor: 0,
            limit: 50,
        },
        0,
    ) else {
        panic!("feed");
    };
    let held = format!("{records:?}");
    for secret in ["buy milk", longest] {
        assert!(!held.contains(secret), "the relay holds {secret}");
    }

    // A write in the clear that bob signed naming an entry from before the epoch began is taken, in a
    // list that is sealed now: he wrote it before he heard of the epoch, and a relay that was given the
    // epoch first judges it as one that was given it after.
    let mut sealed_now = self::relay();
    give_log(&mut sealed_now, 1);
    assert!(matches!(
        ask(
            &mut sealed_now,
            1,
            Request::Append {
                entry: epoch.to_json()
            },
            0
        ),
        Response::Appended { new: true, .. }
    ));
    let mut before_the_epoch = host(Some(2));
    let plain = operation(&mut before_the_epoch, 2, 9, 2000);
    let response = submit(&mut sealed_now, 2, &plain);
    assert!(
        matches!(response, Response::Accepted { new: true, .. }),
        "{response:?}"
    );

    // One that names the entry that began the epoch, or a later one, is in the clear in a list that
    // is sealed: refused.
    let after_the_epoch = host(Some(2));
    after_the_epoch.append(&epoch).unwrap();
    let mut value: Value = serde_json::from_str(&write(2, 10, 3000)).unwrap();
    let signing = message::signing(LIST, "todo.write", &value)
        .unwrap()
        .unwrap();
    let (next, log_hash) = after_the_epoch.head();
    let link = OperationLink {
        seq: 1,
        prev: message::NO_OPERATION,
        log: next - 1,
        log_hash,
    };
    let bytes = message::operation_bytes(&signing.message, &link);
    link.write_into(&mut value).unwrap();
    value["sig"] = Value::String(sce_rust_trust::encoding::encode(&person(2).sign(&bytes)));
    let response = submit(
        &mut relay,
        2,
        &("todo.write".to_string(), value.to_string()),
    );
    assert_eq!(refused(&response), &Code::Invalid);
    let reason = match response {
        Response::Refused { reason, .. } => reason,
        other => panic!("{other:?}"),
    };
    assert!(reason.contains("not sealed"), "{reason}");
}

#[test]
fn a_relay_that_restarts_holds_what_it_held_and_a_tampered_store_does_not_start_it() {
    let dir = std::env::temp_dir().join(format!(
        "sce-relay-restart-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut alice = host(Some(1));
    let op = operation(&mut alice, 1, 1, 1000);
    {
        let mut relay = relay_over(FileStore::open(&dir).unwrap(), Limits::default());
        give_log(&mut relay, 1);
        assert!(matches!(
            submit(&mut relay, 1, &op),
            Response::Accepted { new: true, .. }
        ));
    }
    {
        let mut again = relay_over(FileStore::open(&dir).unwrap(), Limits::default());
        assert_eq!(again.records(LIST), 5);
        assert_eq!(
            submit(&mut again, 2, &op),
            Response::Accepted {
                new: false,
                cursor: 5
            },
            "held already"
        );
    }

    // One byte of what alice signed is changed on the disk.
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(&file).unwrap();
    let tampered = text.replacen("\\\"value\\\":\\\"1\\\"", "\\\"value\\\":\\\"2\\\"", 1);
    assert_ne!(tampered, text);
    std::fs::write(&file, tampered).unwrap();
    let error = Relay::open(
        FileStore::open(&dir).unwrap(),
        authority(),
        Limits::default(),
    )
    .err()
    .expect("a relay does not start from what its members did not sign");
    assert!(error.to_string().contains("does not verify"), "{error}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A store that cannot write down a record: the relay takes nothing more rather than hold what it
/// cannot keep, and says so.
#[test]
fn a_relay_that_cannot_keep_a_record_takes_nothing_more() {
    struct Failing {
        inner: MemoryStore,
        fail: Arc<Mutex<bool>>,
    }
    impl Store for Failing {
        fn load(&mut self) -> std::io::Result<Vec<(String, Record)>> {
            self.inner.load()
        }
        fn append(&mut self, list: &str, record: &Record) -> std::io::Result<()> {
            if *self.fail.lock().unwrap() {
                return Err(std::io::Error::other("the disk is full"));
            }
            self.inner.append(list, record)
        }
    }
    let fail = Arc::new(Mutex::new(false));
    let mut relay = relay_over(
        Failing {
            inner: MemoryStore::new(),
            fail: fail.clone(),
        },
        Limits::default(),
    );
    give_log(&mut relay, 1);
    *fail.lock().unwrap() = true;
    let mut alice = host(Some(1));
    let response = submit(&mut relay, 1, &operation(&mut alice, 1, 1, 1000));
    assert_eq!(refused(&response), &Code::Unavailable);
    *fail.lock().unwrap() = false;
    assert_eq!(
        refused(&ask(&mut relay, 1, Request::Heads, 0)),
        &Code::Unavailable,
        "it does not go on holding what it did not keep"
    );
}

/// Two hosts that never meet: what alice's courier gives the relay, bob's takes out, checked as it
/// would be from anyone, and the machine is told each operation once.
#[test]
fn a_host_gives_a_relay_what_it_lacks_and_is_given_what_it_lacks() {
    let relay = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let link = |node: u64| DirectLink::new(relay.clone(), token(node), clock.clone());

    let alice = host(Some(1));
    let mut alice_sealer = alice.clone();
    let mut alice_courier = Courier::new(alice.clone(), link(1));
    // alice's host holds the log and has made two operations; the relay holds nothing.
    alice_sealer
        .seal("#server", "todo.write", &write(1, 1, 1000))
        .unwrap();
    alice_sealer
        .seal("#server", "todo.write", &write(1, 2, 1001))
        .unwrap();
    let round = alice_courier.sync().unwrap();
    assert_eq!(round.pushed, 4 + 2, "the log and the two operations");
    assert!(round.refused.is_empty(), "{:?}", round.refused);

    let bob = Trust::open(LIST);
    bob.set_identity(2, person(2));
    // bob was told only the genesis, which is all an invitation carries.
    bob.append(&entries()[0]).unwrap();
    let mut bob_courier = Courier::new(bob.clone(), link(2));
    let round = bob_courier.sync().unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);
    assert!(!round.stalled);
    let writes: Vec<&(String, String)> = round
        .events
        .iter()
        .filter(|(event, _)| event == "todo.write")
        .collect();
    assert_eq!(writes.len(), 2);
    let told: Value = serde_json::from_str(&writes[0].1).unwrap();
    assert_eq!(
        told["nodeId"], "1",
        "the machine is told the write as its author made it"
    );
    assert!(
        round.events.iter().any(|(event, _)| event == "todo.role"),
        "and the roster the log makes"
    );

    // A second round has nothing new for the machine.
    let again = bob_courier.sync().unwrap();
    assert!(
        again.events.iter().all(|(event, _)| event != "todo.write"),
        "{:?}",
        again.events
    );
    // A machine that lost its state is told the list again.
    let replay = bob_courier.resync().unwrap();
    assert_eq!(
        replay
            .events
            .iter()
            .filter(|(event, _)| event == "todo.write")
            .count(),
        2
    );
}

/// A relay that withholds an operation is seen by the members and routed around: bob's host takes
/// alice's checkpoint from the relay that hides the operation, sees what it lacks, and the relay
/// that does not hide it gives it.
#[test]
fn a_relay_that_withholds_is_seen_and_routed_around_by_a_second_relay() {
    struct Hiding<L: Link> {
        inner: L,
        hide_seq: &'static str,
    }
    impl<L: Link> Link for Hiding<L> {
        fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
            let response = self.inner.call(request)?;
            Ok(match response {
                Response::Feed {
                    records,
                    next,
                    more,
                } => Response::Feed {
                    records: records
                        .into_iter()
                        .filter(|record| {
                            !matches!(record, Record::Operation { data, .. }
                                if serde_json::from_str::<Value>(data).unwrap()["seq"]
                                    == self.hide_seq)
                        })
                        .collect(),
                    next,
                    more,
                },
                other => other,
            })
        }
    }

    let honest = Arc::new(Mutex::new(relay()));
    let hiding = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let link = |relay: &Arc<Mutex<Relay<MemoryStore>>>, node: u64| {
        DirectLink::new(relay.clone(), token(node), clock.clone())
    };

    let alice = host(Some(1));
    let mut sealer = alice.clone();
    for n in 1..=3 {
        sealer
            .seal("#server", "todo.write", &write(1, n, 1000 + n as i64))
            .unwrap();
    }
    // alice gives both relays everything; the second one will not pass the second operation on.
    for relay in [&honest, &hiding] {
        Courier::new(alice.clone(), link(relay, 1)).sync().unwrap();
    }
    let checkpoint = alice.checkpoint(30_000, 7).unwrap();
    Courier::new(alice.clone(), link(&hiding, 1))
        .sync()
        .unwrap();

    let bob = host(Some(2));
    let mut through_the_hiding = Courier::new(
        bob.clone(),
        Hiding {
            inner: link(&hiding, 2),
            hide_seq: "2",
        },
    );
    let round = through_the_hiding.sync().unwrap();
    let writes = round
        .events
        .iter()
        .filter(|(e, _)| e == "todo.write")
        .count();
    assert_eq!(writes, 2, "the relay passed on 1 and 3");
    assert_eq!(bob.heads()[0].seq, 1, "the chain is in full only to 1");
    assert_eq!(
        bob.take_checkpoint(&checkpoint),
        Ok(false),
        "the relay that hides the operation passed the checkpoint on, and bob holds it"
    );
    assert_eq!(bob.divergences().len(), 1, "alice's checkpoint names 3");

    // The other relay gives the one that is missing, and the machine is told it.
    let mut through_the_honest = Courier::new(bob.clone(), link(&honest, 2));
    let round = through_the_honest.sync().unwrap();
    let writes: Vec<_> = round
        .events
        .iter()
        .filter(|(e, _)| e == "todo.write")
        .collect();
    assert_eq!(writes.len(), 1, "only the operation the host lacked");
    assert!(bob.divergences().is_empty());
    assert_eq!(bob.heads()[0].seq, 3);
}

/// An operation that a host refused because its device was taken off before it, and that an amendment
/// then raised the last accepted to include, is taken in when the courier reads the relay again.
#[test]
fn an_amendment_makes_a_courier_read_again_what_it_refused() {
    let relay_b = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    {
        // The relay that holds both of bob's operations has not heard of his removal yet.
        let mut b = relay_b.lock().unwrap();
        give_log(&mut b, 1);
    }
    let mut bob = host(Some(2));
    let ops: Vec<(String, String)> = (1..=2)
        .map(|n| operation(&mut bob, 2, n, 1000 + n as i64))
        .collect();
    for op in &ops {
        let mut b = relay_b.lock().unwrap();
        assert!(matches!(
            submit(&mut b, 2, op),
            Response::Accepted { new: true, .. }
        ));
    }

    // Alice holds both too, and her checkpoint, made before the removal, names bob's chain to 2.
    let mut alice = host(Some(1));
    for (event, data) in &ops {
        alice.admit("relay", event, data).unwrap();
    }
    let checkpoint = alice.checkpoint(30_000, 7).unwrap();

    // The administrator takes bob off at his first operation; alice and carol hold that entry.
    let carol = host(Some(3));
    let removal = host(None)
        .draft(Body::Grant {
            node: 2,
            role: 0,
            key: person(2).public(),
            final_seq: Some(1),
        })
        .sign(&admin())
        .finish();
    alice.append(&removal).unwrap();
    carol.append(&removal).unwrap();

    // Carol's courier gives that relay the removal, and reads: bob's first operation is taken in, and
    // his second is refused, as past what the administrators accepted.
    let mut courier = Courier::new(
        carol.clone(),
        DirectLink::new(relay_b.clone(), token(3), clock),
    );
    let round = courier.sync().unwrap();
    assert_eq!(
        round
            .events
            .iter()
            .filter(|(event, _)| event == "todo.write")
            .count(),
        1
    );
    assert!(
        round.refused.iter().any(|reason| reason.contains("past 1")),
        "{:?}",
        round.refused
    );
    assert_eq!(carol.last_seq(2), 1);

    // The administrator amends the removal on the evidence of alice's checkpoint, and carol's courier
    // reads the relay again and takes the second operation in.
    let amendment = alice
        .draft(Body::Amend {
            node: 2,
            final_seq: 2,
            evidence: vec![checkpoint],
        })
        .sign(&admin())
        .finish();
    carol.append(&amendment).unwrap();
    let round = courier.sync().unwrap();
    let writes: Vec<&(String, String)> = round
        .events
        .iter()
        .filter(|(event, _)| event == "todo.write")
        .collect();
    assert_eq!(writes.len(), 1, "{:?}", round.events);
    assert_eq!(carol.last_seq(2), 2);
}

/// A range of operations is given up to the relay's bound, and a range of exactly the bound whole.
#[test]
fn a_range_of_operations_is_given_up_to_the_relays_bound_and_no_further() {
    let limits = Limits {
        max_range: 3,
        ..Limits::default()
    };
    let mut relay = relay_over(MemoryStore::new(), limits);
    give_log(&mut relay, 1);
    let mut alice = host(Some(1));
    for id in 1..=5u64 {
        let op = operation(&mut alice, 1, id, 1000 + id as i64);
        assert!(matches!(
            submit(&mut relay, 1, &op),
            Response::Accepted { new: true, .. }
        ));
    }
    let count = |relay: &mut Relay<MemoryStore>, from: u64, to: u64| match ask(
        relay,
        2,
        Request::Operations { node: 1, from, to },
        0,
    ) {
        Response::Operations { operations } => operations.len(),
        other => panic!("operations: {other:?}"),
    };
    assert_eq!(
        count(&mut relay, 1, 5),
        3,
        "five were asked for, three is the bound"
    );
    assert_eq!(
        count(&mut relay, 1, 3),
        3,
        "exactly the bound is given whole"
    );
    assert_eq!(
        count(&mut relay, 2, 5),
        3,
        "a range that starts later is bounded the same"
    );
    assert_eq!(
        count(&mut relay, 4, 5),
        2,
        "a range inside the bound is given as asked"
    );
}

/// A checkpoint the relay kept is read back through the check it was taken in by: one that was
/// changed on the disk does not start the relay.
#[test]
fn a_relay_that_restarts_does_not_start_from_a_checkpoint_that_was_changed() {
    let dir = std::env::temp_dir().join(format!(
        "sce-relay-checkpoint-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let alice = host(Some(1));
    let checkpoint = alice.checkpoint(30_000, 7).unwrap();
    {
        let mut relay = relay_over(FileStore::open(&dir).unwrap(), Limits::default());
        give_log(&mut relay, 1);
        let response = ask(
            &mut relay,
            1,
            Request::Checkpoint {
                checkpoint: checkpoint.to_json(),
            },
            0,
        );
        assert!(
            matches!(response, Response::Accepted { new: true, .. }),
            "{response:?}"
        );
    }
    // What it kept starts it again.
    drop(relay_over(
        FileStore::open(&dir).unwrap(),
        Limits::default(),
    ));

    // One number of what alice signed is changed on the disk.
    let file = std::fs::read_dir(&dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(&file).unwrap();
    let tampered = text.replacen("\\\"digest\\\":\\\"7\\\"", "\\\"digest\\\":\\\"8\\\"", 1);
    assert_ne!(
        tampered, text,
        "the checkpoint is in the store as it was sent"
    );
    std::fs::write(&file, tampered).unwrap();
    let error = Relay::open(
        FileStore::open(&dir).unwrap(),
        authority(),
        Limits::default(),
    )
    .err()
    .expect("a relay does not start from a checkpoint its device did not sign");
    assert!(error.to_string().contains("does not verify"), "{error}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A device whose key is on two hosts that have not heard of each other signs two operations at one
/// number. The host that finds out first gives the relay both, as proof, and a host that read only one
/// of them from the relay finds out from the relay too: the proof is not any host's own knowledge.
#[test]
fn the_proof_that_a_device_signed_conflicting_operations_goes_through_a_relay_to_a_host_that_saw_one(
) {
    let relay = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let link = |node: u64| DirectLink::new(relay.clone(), token(node), clock.clone());

    let first = host(Some(1));
    let second = host(Some(1));
    first
        .clone()
        .seal("#server", "todo.write", &write(1, 1, 1000))
        .unwrap();
    second
        .clone()
        .seal("#server", "todo.write", &write(1, 2, 1001))
        .unwrap();
    Courier::new(first.clone(), link(1)).sync().unwrap();

    // The second host reads the relay and is given the first's operation: its own and that one
    // conflict. The next round gives the relay the one the relay lacks, as proof.
    let mut second_courier = Courier::new(second.clone(), link(1));
    second_courier.sync().unwrap();
    assert_eq!(second.equivocating(), vec![(1, 1)]);
    second_courier.sync().unwrap();

    // bob was given none of it by anyone else: the relay gives him both, and he finds it.
    let bob = host(Some(2));
    let mut bob_courier = Courier::new(bob.clone(), link(2));
    let round = bob_courier.sync().unwrap();
    assert_eq!(bob.equivocating(), vec![(1, 1)]);
    assert!(
        bob.all_operations().is_empty(),
        "nothing of the device from 1"
    );
    assert_eq!(
        round.evicted,
        vec![(1, 1)],
        "his machine was told the first, and forgets it"
    );
    // The machine was told the one that held when it was taken in, and is told nothing of the one that
    // is held only as proof.
    let told = round
        .events
        .iter()
        .filter(|(event, _)| event == "todo.write")
        .count();
    assert_eq!(told, 1, "{:?}", round.events);
}

/// The relay reads none of the MLS messages and bounds what it keeps of them: how big one is, that it
/// says something, how many key packages a device leaves, that a package is kept once, that a commit
/// is the one the log names, and that a Welcome is for an epoch the log began.
#[test]
fn a_relay_keeps_the_mls_messages_of_a_list_within_its_bounds_and_the_logs_word() {
    use sce_rust_trust::encoding;
    use sha2::{Digest, Sha256};

    let limits = Limits {
        max_group_message_bytes: 16,
        max_key_packages_per_node: 2,
        ..Limits::default()
    };
    let mut relay = relay_over(MemoryStore::new(), limits);
    give_log(&mut relay, 1);
    let commit = vec![7u8; 10];
    let named: [u8; 32] = Sha256::digest(&commit).into();
    let entry = host(None)
        .draft(Body::MlsEpoch {
            epoch: 1,
            commit: named,
        })
        .sign(&admin())
        .finish();
    let response = ask(
        &mut relay,
        1,
        Request::Append {
            entry: entry.to_json(),
        },
        0,
    );
    assert!(
        matches!(response, Response::Appended { new: true, .. }),
        "{response:?}"
    );

    let package = |relay: &mut Relay<MemoryStore>, node: u64, bytes: &[u8]| {
        ask(
            relay,
            node,
            Request::PublishKeyPackage {
                data: encoding::encode(bytes),
            },
            0,
        )
    };
    let before = relay.records(LIST);

    // A message is at most the bound, and says something.
    assert!(matches!(
        package(&mut relay, 3, &[1u8; 16]),
        Response::Accepted { new: true, .. }
    ));
    assert_eq!(
        refused(&package(&mut relay, 3, &[1u8; 17])),
        &Code::TooLarge
    );
    assert_eq!(refused(&package(&mut relay, 3, &[])), &Code::Invalid);

    // A device leaves at most two, and one it left is kept once.
    assert!(matches!(
        package(&mut relay, 3, &[2u8; 8]),
        Response::Accepted { new: true, .. }
    ));
    assert_eq!(refused(&package(&mut relay, 3, &[3u8; 8])), &Code::TooLarge);
    let held = relay.records(LIST);
    assert_eq!(held, before + 2);
    assert!(
        matches!(
            package(&mut relay, 3, &[2u8; 8]),
            Response::Accepted { new: false, .. }
        ),
        "a package held already is not kept again, and is not counted against the device"
    );
    assert_eq!(relay.records(LIST), held);
    // Another device's allowance of packages is its own.
    assert!(matches!(
        package(&mut relay, 2, &[4u8; 8]),
        Response::Accepted { new: true, .. }
    ));

    // A commit is the one the log names for its epoch.
    let commit_of = |relay: &mut Relay<MemoryStore>, epoch: u32, bytes: &[u8]| {
        ask(
            relay,
            1,
            Request::PublishCommit {
                epoch,
                data: encoding::encode(bytes),
            },
            0,
        )
    };
    assert_eq!(
        refused(&commit_of(&mut relay, 1, &[8u8; 10])),
        &Code::Invalid,
        "not the commit the log names"
    );
    assert_eq!(
        refused(&commit_of(&mut relay, 2, &commit)),
        &Code::Behind,
        "the log names no commit for epoch 2"
    );
    assert!(matches!(
        commit_of(&mut relay, 1, &commit),
        Response::Accepted { new: true, .. }
    ));
    assert!(matches!(
        commit_of(&mut relay, 1, &commit),
        Response::Accepted { new: false, .. }
    ));

    // A Welcome is for an epoch the log began.
    let welcome = |relay: &mut Relay<MemoryStore>, epoch: u32| {
        ask(
            relay,
            1,
            Request::PublishWelcome {
                epoch,
                node: 3,
                data: encoding::encode(&[9u8; 12]),
            },
            0,
        )
    };
    assert_eq!(refused(&welcome(&mut relay, 2)), &Code::Behind);
    assert!(matches!(
        welcome(&mut relay, 1),
        Response::Accepted { new: true, .. }
    ));
}
