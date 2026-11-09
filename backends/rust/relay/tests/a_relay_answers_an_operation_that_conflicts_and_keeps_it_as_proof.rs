// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a relay does with an operation of one author at a number where it holds another, and with
// every other operation a device set up again with no state could give it (ADR 0083 section 4):
// the same operation given again, the next one, one past a gap, a second change of the author
// writing over a write it wrote over already, another operation at a number the relay holds one at,
// and a third
// there. Each is given by the author's own session and by a member's courier, in each version of
// the protocol the relay speaks, and held to one oracle written apart from the relay: what it
// keeps, what it answers, what its heads say of the author after, what it logs, and what a host
// that reads it from the start finds. An operation that conflicts is kept as proof whoever gives
// it, and answered `conflict` with what the relay held to a client of version 2; a client of
// version 1 is answered `accepted`, as a relay of version 1 answered it. What a device's side does
// with the answer is `a_device_told_its_operation_conflicts_freezes_at_once`.

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_relay::logs::{capture, list_digest};
use sce_rust_relay::protocol::{CONFLICT_SINCE, OLDEST_VERSION, VERSION};
use sce_rust_relay::{Authority, Code, Limits, MemoryStore, Record, Relay, Request, Response};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The entries of the list: alice and bob write, carol reads.
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
    for (node, role) in [(ALICE, 2u8), (BOB, 2), (CAROL, 1)] {
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

/// A host that holds the log, as the person of `own`.
fn host(own: u64) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries() {
        trust.append(&entry).unwrap();
    }
    trust.set_identity(own, person(own));
    trust
}

fn authority() -> Authority {
    Authority::new([7; 32], "relay-a")
}

/// A change of alice's stamped at `wall` of one write, to field 3 of row `id`.
fn write(id: u32, wall: i64) -> String {
    json!({"by": ALICE.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": id.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// Alice's change stamped at `at` that writes over her write of row `id` stamped at `wall`.
fn note(id: u32, wall: i64, at: i64) -> String {
    json!({"by": ALICE.to_string(), "wallTime": at.to_string(), "counter": 0, "ops": [
        {"kind": 1, "id": id, "field": 3, "value": "0", "text": "",
         "wallTime": wall.to_string(), "counter": 0, "nodeId": ALICE.to_string()}]})
    .to_string()
}

/// What `host` sends of `event`, signed: the operation as it travels.
fn sealed(host: &Trust, event: &str, data: &str) -> (String, String) {
    let sent = host
        .clone()
        .seal("#hub", event, data)
        .unwrap()
        .expect("an event that is signed");
    (event.to_string(), sent)
}

fn submit(op: &(String, String)) -> Request {
    Request::Submit {
        event: op.0.clone(),
        data: op.1.clone(),
    }
}

/// What the relay answers the device of `node` that asks `request` in `version` of the protocol,
/// and the version the answer is written in.
fn ask(
    relay: &mut Relay<MemoryStore>,
    node: u64,
    request: &Request,
    version: u64,
) -> (Response, u64) {
    let token = authority().issue(LIST, node, u64::MAX);
    let text = relay.handle(&token, &request.to_json_in(version), 0);
    let written = serde_json::from_str::<Value>(&text).unwrap()["v"]
        .as_u64()
        .expect("every answer names its version");
    (Response::from_json(&text).unwrap(), written)
}

fn parsed(data: &str) -> Value {
    serde_json::from_str(data).unwrap()
}

/// Operations as values, in an order that does not depend on the order they were given in.
fn sorted(operations: &[(String, String)]) -> Vec<(String, Value)> {
    let mut all: Vec<(String, Value)> = operations
        .iter()
        .map(|(event, data)| (event.clone(), parsed(data)))
        .collect();
    all.sort_by_key(|(event, data)| format!("{event}{data}"));
    all
}

/// A relay that holds the log and three operations of alice's: a change that writes (1), her
/// change that writes it over (2), and a second change that writes (3).
struct World {
    relay: Relay<MemoryStore>,
    alice: Trust,
    held: Vec<(String, String)>,
}

fn world() -> World {
    let mut relay = Relay::open(MemoryStore::new(), authority(), Limits::default()).unwrap();
    for entry in entries() {
        let append = Request::Append {
            entry: entry.to_json(),
        };
        let (answer, _) = ask(&mut relay, ALICE, &append, VERSION);
        assert!(
            matches!(answer, Response::Appended { new: true, .. }),
            "{answer:?}"
        );
    }
    let alice = host(ALICE);
    let held = vec![
        sealed(&alice, CHANGE, &write(1, 1000)),
        sealed(&alice, CHANGE, &note(1, 1000, 1001)),
        sealed(&alice, CHANGE, &write(2, 1002)),
    ];
    for op in &held {
        let (answer, _) = ask(&mut relay, ALICE, &submit(op), VERSION);
        assert!(
            matches!(answer, Response::Accepted { new: true, .. }),
            "{answer:?}"
        );
    }
    World { relay, alice, held }
}

/// What a device of alice's set up again from her seed with no state signs, having been given only
/// the operations of hers at the places `taken` of what the relay holds: it numbers its operation
/// after the last of those, where the relay may hold another.
fn lost_state(world: &World, taken: &[usize], event: &str, data: &str) -> (String, String) {
    let copy = host(ALICE);
    for at in taken {
        let (event, data) = &world.held[*at];
        copy.take_operation(event, data).unwrap();
    }
    sealed(&copy, event, data)
}

/// What is given the relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Given {
    /// Alice's third operation, which the relay holds, given again.
    Again,
    /// Her fourth: the number after the relay's head of her.
    Next,
    /// Her fifth, the fourth never given: past a gap.
    PastAGap,
    /// A third operation of hers with other bytes than the relay's third.
    OtherAtANumber,
    /// Another third with other bytes, given once the relay holds two.
    ThirdAtANumber,
    /// A fourth that is a second change of hers over the write her second operation wrote over
    /// (docs/adr/0085, section 7.4).
    SecondNote,
}

const GIVEN: [Given; 6] = [
    Given::Again,
    Given::Next,
    Given::PastAGap,
    Given::OtherAtANumber,
    Given::ThirdAtANumber,
    Given::SecondNote,
];

/// What the relay is to do with what is given, written from the rules and not from the relay:
/// whether it keeps one record more, whether what it was given conflicts with what it held of
/// alice, and the head of her chain the relay names after.
struct Oracle {
    kept: bool,
    conflict: bool,
    head: u64,
}

fn oracle(given: Given) -> Oracle {
    let (kept, conflict, head) = match given {
        // Held already: nothing kept, the head where it was.
        Given::Again => (false, false, 3),
        Given::Next => (true, false, 4),
        // Kept; the head is where the chain is whole from 1.
        Given::PastAGap => (true, false, 3),
        // Two changes of one author whose write-overs name one dot are two true statements, links
        // of one chain.
        Given::SecondNote => (true, false, 4),
        // Two operations at 3: kept as proof, and the head stops below them.
        Given::OtherAtANumber | Given::ThirdAtANumber => (true, true, 2),
    };
    Oracle {
        kept,
        conflict,
        head,
    }
}

/// The operation given, its number, and what the relay holds that it conflicts with. A third at a
/// number is given after alice's own session gave the relay the other.
fn make(world: &mut World, given: Given) -> ((String, String), u64, Vec<(String, String)>) {
    match given {
        Given::Again => (world.held[2].clone(), 3, Vec::new()),
        Given::Next => (sealed(&world.alice, CHANGE, &write(3, 1003)), 4, Vec::new()),
        Given::PastAGap => {
            let _never_given = sealed(&world.alice, CHANGE, &write(3, 1003));
            (sealed(&world.alice, CHANGE, &write(4, 1004)), 5, Vec::new())
        }
        Given::OtherAtANumber => {
            let other = lost_state(world, &[0, 1], CHANGE, &write(9, 1009));
            (other, 3, vec![world.held[2].clone()])
        }
        Given::ThirdAtANumber => {
            let other = lost_state(world, &[0, 1], CHANGE, &write(9, 1009));
            let (answer, _) = ask(&mut world.relay, ALICE, &submit(&other), VERSION);
            assert!(
                matches!(
                    answer,
                    Response::Refused {
                        code: Code::Conflict { .. },
                        ..
                    }
                ),
                "{answer:?}"
            );
            let third = lost_state(world, &[0, 1], CHANGE, &write(10, 1010));
            (third, 3, vec![world.held[2].clone(), other])
        }
        Given::SecondNote => {
            let again = lost_state(world, &[0, 2], CHANGE, &note(1, 1000, 1005));
            (again, 4, Vec::new())
        }
    }
}

/// Every operation of the cross product, given by alice's own session and by bob's courier, in
/// every version the relay speaks, against the oracle.
#[test]
fn every_operation_a_device_could_give_is_kept_answered_and_logged_as_the_oracle_says() {
    capture::start();
    let digest = list_digest(LIST);
    for given in GIVEN {
        for by in [ALICE, BOB] {
            for version in OLDEST_VERSION..=VERSION {
                let case = format!("{given:?} by node {by} in version {version}");
                let expected = oracle(given);
                let mut world = world();
                let (op, seq, conflicts_with) = make(&mut world, given);
                let before = world.relay.records(LIST) as u64;
                let _ = capture::taken();

                let (answer, written) = ask(&mut world.relay, by, &submit(&op), version);
                let lines: Vec<String> = capture::taken()
                    .into_iter()
                    .map(|taken| taken.line)
                    .collect();

                // What it keeps.
                let after = before + u64::from(expected.kept);
                assert_eq!(world.relay.records(LIST) as u64, after, "{case}");
                let range = Request::Operations {
                    node: ALICE,
                    from: seq,
                    to: seq,
                };
                let (Response::Operations { operations }, _) =
                    ask(&mut world.relay, CAROL, &range, VERSION)
                else {
                    panic!("{case}: the relay gives a range");
                };
                assert!(
                    operations
                        .iter()
                        .any(|(_, data)| parsed(data) == parsed(&op.1)),
                    "{case}: the relay holds what it was given at {seq}"
                );
                if expected.conflict {
                    assert_eq!(
                        sorted(&operations),
                        sorted(&[conflicts_with.clone(), vec![op.clone()]].concat()),
                        "{case}: what it was given is kept beside what it held"
                    );
                }

                // What it answers, in the version it was asked in.
                assert_eq!(written, version, "{case}");
                let told = expected.conflict && version >= CONFLICT_SINCE;
                if told {
                    let Response::Refused {
                        code:
                            Code::Conflict {
                                node,
                                seq: at,
                                frozen,
                                cursor,
                                held,
                            },
                        reason,
                    } = &answer
                    else {
                        panic!("{case}: answered {answer:?}");
                    };
                    assert_eq!(
                        (*node, *at, *frozen, *cursor),
                        (ALICE, seq, 3, after),
                        "{case}"
                    );
                    assert_eq!(sorted(held), sorted(&conflicts_with), "{case}");
                    assert!(reason.contains("kept as proof"), "{case}: {reason}");
                } else {
                    assert_eq!(
                        answer,
                        Response::Accepted {
                            new: expected.kept,
                            cursor: after
                        },
                        "{case}"
                    );
                }

                // What its heads say of alice.
                let (Response::Heads { heads, .. }, _) =
                    ask(&mut world.relay, CAROL, &Request::Heads, VERSION)
                else {
                    panic!("{case}: the relay gives its heads");
                };
                let head = heads
                    .iter()
                    .find(|head| head.node == ALICE)
                    .map(|head| head.seq);
                assert_eq!(head, Some(expected.head), "{case}");

                // What it logs: one record of the conflict, and the answer of a request answered
                // as asked.
                let answered =
                    format!("event=answered op=submit list={digest} node={by} answer=accepted");
                let expected_lines = if expected.conflict {
                    let froze = given == Given::OtherAtANumber;
                    let word = if told { "conflict" } else { "accepted" };
                    let kept = format!(
                        "event=conflict_kept list={digest} node={ALICE} seq={seq} at=3 frozen=3 \
                         froze={froze} by={by} cursor={after} version={version} answer={word}"
                    );
                    if told {
                        vec![kept]
                    } else {
                        vec![kept, answered]
                    }
                } else {
                    vec![answered]
                };
                assert_eq!(lines, expected_lines, "{case}");
                for line in &lines {
                    assert!(
                        !line.contains(LIST),
                        "{case}: a list's name in a record: {line}"
                    );
                }

                // What a host that reads the relay from its start finds.
                let reader = host(CAROL);
                let feed = Request::Feed {
                    cursor: 0,
                    limit: 500,
                };
                let (Response::Feed { records, more, .. }, _) =
                    ask(&mut world.relay, CAROL, &feed, VERSION)
                else {
                    panic!("{case}: the relay gives its feed");
                };
                assert!(!more, "{case}");
                for record in records {
                    match record {
                        Record::Entry(text) => {
                            reader.append(&Entry::from_json(&text).unwrap()).unwrap();
                        }
                        Record::Operation { event, data } => {
                            reader.take_operation(&event, &data).unwrap();
                        }
                        other => panic!("{case}: {other:?}"),
                    }
                }
                let frozen = if expected.conflict {
                    vec![(ALICE, 3)]
                } else {
                    Vec::new()
                };
                assert_eq!(reader.equivocating(), frozen, "{case}");
            }
        }
    }
}

/// A request of a version the relay does not speak, and a text that is no request, are refused
/// in the oldest version it speaks, which every client reads; a request it speaks is answered in
/// its own.
#[test]
fn a_request_is_answered_in_its_version_and_one_the_relay_does_not_speak_in_the_oldest() {
    let mut world = world();
    let token = authority().issue(LIST, CAROL, u64::MAX);
    let version_of = |text: &str| parsed(text)["v"].as_u64();
    for version in OLDEST_VERSION..=VERSION {
        let text = world
            .relay
            .handle(&token, &Request::Heads.to_json_in(version), 0);
        assert_eq!(version_of(&text), Some(version), "{text}");
    }
    for unread in [
        Request::Heads.to_json_in(VERSION + 1),
        Request::Heads.to_json_in(0),
        "not json".to_string(),
    ] {
        let text = world.relay.handle(&token, &unread, 0);
        assert_eq!(version_of(&text), Some(OLDEST_VERSION), "{unread}: {text}");
        assert!(
            matches!(
                Response::from_json(&text),
                Ok(Response::Refused {
                    code: Code::Invalid,
                    ..
                })
            ),
            "{unread}: {text}"
        );
    }
}
