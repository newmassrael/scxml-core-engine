// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A courier's round as a state machine with no link: the host is asked for one request at a time
// and gives the answer back, whenever it has one. What is asked of it here: that a round driven by
// hand makes the requests a courier makes, in the same order, and comes to the same round, over one
// history of the list (alice's log, two writes of hers, the note that the second wrote over the
// first, and her checkpoint) in two universes that never meet; that a round whose answer never
// comes, or whose link fails, leaves the next round to tell the machine what an unbroken round would
// have told it, and nothing twice; and that a round that stalled on a log kept back goes on from
// where it stopped once the log comes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_relay::{
    Authority, Code, Courier, CourierError, CourierState, Limits, Link, LinkError, MemoryStore,
    Publication, Record, Relay, Request, Response, Round, RoundKind, RoundMachine, Step,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::json;

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const DAVE: u64 = 4;

const WRITE: &str = "record_list.write";
const SUPERSEDE: &str = "record_list.supersede";
const ROLE: &str = "record_list.role";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
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
    for (node, role) in [(ALICE, 2u8), (BOB, 2), (3, 1)] {
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

fn authority() -> Authority {
    Authority::new([7; 32], "relay-a")
}

fn token(node: u64) -> String {
    authority().issue(LIST, node, 1_000_000)
}

/// A write of `value` to field 3 of row 1, stamped by `node` at `wall`, as the record list sends it.
fn write(node: u64, wall: i64, value: i64) -> String {
    json!({"id": 1, "field": 3, "value": value.to_string(), "text": "",
           "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()})
    .to_string()
}

/// The note that `by` wrote over the write `node` stamped at `wall`, in the change stamped `at`.
fn supersede(node: u64, wall: i64, by: u64, at: i64) -> String {
    json!({"id": 1, "field": 3, "wallTime": wall.to_string(), "counter": 0,
           "nodeId": node.to_string(), "by": by.to_string(),
           "atWallTime": at.to_string(), "atCounter": 0})
    .to_string()
}

type Shared = Arc<Mutex<Relay<MemoryStore>>>;

/// One history of the list: a relay that holds alice's log, two writes of hers, the note that the
/// second wrote over the first, and her checkpoint; alice's host; and a host of bob's that was told
/// only the genesis, which is all an invitation carries. Two universes made by this hold the same
/// bytes, so what a courier does in one a machine driven by hand must do in the other.
struct Universe {
    relay: Shared,
    alice: Trust,
    bob: Trust,
}

fn universe(limits: Limits) -> Universe {
    let relay: Shared = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority(), limits).unwrap(),
    ));
    let alice = Trust::open(LIST);
    for entry in entries() {
        alice.append(&entry).unwrap();
    }
    alice.set_identity(ALICE, person(ALICE));
    let mut sealer = alice.clone();
    for (event, data) in [
        (WRITE, write(ALICE, 1000, 1)),
        (WRITE, write(ALICE, 1001, 2)),
        (SUPERSEDE, supersede(ALICE, 1000, ALICE, 1001)),
    ] {
        assert!(sealer.seal("#hub", event, &data).unwrap().is_some());
    }
    alice.checkpoint(30_000, 7).unwrap();
    let mut courier = Courier::new(alice.clone(), link(&relay, ALICE).0);
    let round = courier.sync().unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);
    assert_eq!(relay.lock().unwrap().records(LIST), 4 + 3 + 1);
    Universe {
        relay,
        alice,
        bob: host_told_the_genesis(BOB),
    }
}

fn host_told_the_genesis(node: u64) -> Trust {
    let host = Trust::open(LIST);
    host.append(&entries()[0]).unwrap();
    host.set_identity(node, person(node));
    host
}

/// What the relay answers `node`'s `request`; while `withhold`, the log is kept back from it, in
/// the answer to a request for entries and in the feed, as a relay that is not trusted can.
fn answer(
    relay: &Shared,
    node: u64,
    withhold: bool,
    request: &Request,
) -> Result<Response, LinkError> {
    let text = relay
        .lock()
        .unwrap()
        .handle(&token(node), &request.to_json(), 0);
    let response = Response::from_json(&text).map_err(LinkError)?;
    if !withhold {
        return Ok(response);
    }
    Ok(match (request, response) {
        (Request::Entries { from }, Response::Entries { .. }) => Response::Entries {
            entries: Vec::new(),
            len: *from,
        },
        (
            _,
            Response::Feed {
                records,
                next,
                more,
            },
        ) => Response::Feed {
            records: records
                .into_iter()
                .filter(|record| !matches!(record, Record::Entry(_)))
                .collect(),
            next,
            more,
        },
        (_, other) => other,
    })
}

/// A courier's link to the relay of a universe: it answers as [`answer`] does and keeps every
/// request it carried.
struct TestLink {
    relay: Shared,
    node: u64,
    withhold: Arc<AtomicBool>,
    asked: Arc<Mutex<Vec<Request>>>,
}

impl Link for TestLink {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
        self.asked.lock().unwrap().push(request.clone());
        answer(
            &self.relay,
            self.node,
            self.withhold.load(Ordering::SeqCst),
            request,
        )
    }
}

type Asked = Arc<Mutex<Vec<Request>>>;

/// A link for `node` that keeps nothing back, and what it was asked.
fn link(relay: &Shared, node: u64) -> (TestLink, Asked) {
    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let link = TestLink {
        relay: relay.clone(),
        node,
        withhold: Arc::new(AtomicBool::new(false)),
        asked: asked.clone(),
    };
    (link, asked)
}

fn taken(asked: &Asked) -> Vec<Request> {
    std::mem::take(&mut *asked.lock().unwrap())
}

type Driven = (Vec<Request>, Result<Round, CourierError>, CourierState);

/// Drive `machine` to its end, each request answered by `respond`: the requests it asked in order,
/// its outcome, and the state it leaves.
fn drive(
    mut machine: RoundMachine,
    mut respond: impl FnMut(&Request) -> Result<Response, LinkError>,
) -> Driven {
    let mut asked = Vec::new();
    let outcome = loop {
        match machine.poll() {
            Step::Request(request) => {
                let response = respond(&request);
                asked.push(request);
                machine.respond(response);
            }
            Step::Done(outcome) => break outcome,
        }
    };
    (asked, outcome, machine.into_state())
}

fn named<'a>(round: &'a Round, event: &str) -> Vec<&'a (String, String)> {
    round
        .events
        .iter()
        .filter(|(name, _)| name == event)
        .collect()
}

#[test]
fn a_round_driven_by_hand_asks_and_comes_to_what_a_courier_does() {
    let by_courier = universe(Limits::default());
    let by_hand = universe(Limits::default());
    let (courier_link, asked) = link(&by_courier.relay, BOB);
    let mut courier = Courier::new(by_courier.bob.clone(), courier_link);
    let mut state = CourierState::default();
    let mut rounds = Vec::new();
    for kind in [
        RoundKind::Sync,
        RoundKind::Sync,
        RoundKind::Resync,
        RoundKind::Reread,
    ] {
        let expected = match kind {
            RoundKind::Sync => courier.sync(),
            RoundKind::Resync => courier.resync(),
            RoundKind::Reread => courier.reread(),
        };
        let machine = RoundMachine::start(kind, state, by_hand.bob.clone());
        let (requests, outcome, left) = drive(machine, |request| {
            answer(&by_hand.relay, BOB, false, request)
        });
        assert_eq!(requests, taken(&asked), "{kind:?}");
        assert_eq!(outcome, expected, "{kind:?}");
        assert_eq!(left.cursor(), courier.cursor(), "{kind:?}");
        assert_eq!(left.untold(), 0, "{kind:?}");
        rounds.push(outcome.unwrap());
        state = left;
    }

    // The first round tells bob's machine the roster, then alice's writes and her note, in order.
    let first = &rounds[0];
    assert_eq!(named(first, ROLE).len(), 3, "{:?}", first.events);
    let operations: Vec<&str> = first
        .events
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| *name != ROLE)
        .collect();
    assert_eq!(operations, vec![WRITE, WRITE, SUPERSEDE]);
    assert!(first.refused.is_empty(), "{:?}", first.refused);
    assert!(!first.stalled);
    // The second has nothing new; a resync tells everything again; a reread tells nothing held.
    assert!(rounds[1].events.is_empty(), "{:?}", rounds[1].events);
    assert_eq!(named(&rounds[2], WRITE).len(), 2);
    assert_eq!(named(&rounds[2], SUPERSEDE).len(), 1);
    assert!(named(&rounds[3], WRITE).is_empty());

    // A checkpoint and a key package are published the same way by both.
    let expected = courier.publish_checkpoint(30_000, 7);
    let machine = RoundMachine::publish(
        &Publication::Checkpoint {
            before: 30_000,
            digest: 7,
        },
        state,
        by_hand.bob.clone(),
    );
    let (requests, outcome, state) = drive(machine, |request| {
        answer(&by_hand.relay, BOB, false, request)
    });
    let wanted = taken(&asked);
    assert_eq!(requests, wanted);
    assert!(
        matches!(wanted.last(), Some(Request::Checkpoint { .. })),
        "{wanted:?}"
    );
    assert_eq!(outcome.map(|_| ()), expected);
    assert_eq!(expected, Ok(()));

    let expected = courier.publish_key_package(b"a key package");
    let machine = RoundMachine::publish(
        &Publication::KeyPackage(b"a key package".to_vec()),
        state,
        by_hand.bob.clone(),
    );
    let (requests, outcome, _) = drive(machine, |request| {
        answer(&by_hand.relay, BOB, false, request)
    });
    assert_eq!(requests, taken(&asked));
    assert_eq!(requests.len(), 1, "a key package is one request");
    assert_eq!(outcome.map(|_| ()), expected);
}

#[test]
fn a_publication_says_what_the_relay_answered() {
    assert_eq!(
        Publication::check(Response::Accepted {
            new: true,
            cursor: 3
        }),
        Ok(())
    );
    assert_eq!(
        Publication::check(Response::refused(Code::Forbidden, "no")),
        Err(CourierError::Relay {
            code: Code::Forbidden,
            reason: "no".to_string()
        })
    );
    assert!(matches!(
        Publication::check(Response::Appended { new: true, len: 1 }),
        Err(CourierError::Unexpected(_))
    ));
    // A host that has no person makes no checkpoint, and asks nothing.
    let nobody = Trust::open(LIST);
    for entry in entries() {
        nobody.append(&entry).unwrap();
    }
    let mut machine = RoundMachine::publish(
        &Publication::Checkpoint {
            before: 1,
            digest: 1,
        },
        CourierState::default(),
        nobody,
    );
    assert!(matches!(
        machine.poll(),
        Step::Done(Err(CourierError::Unexpected(_)))
    ));
}

/// What a host that composes rounds holds back from one is handed over by the next round, first;
/// a resync drops the events and messages it reads again, and never what the machine is to be
/// rebuilt by.
#[test]
fn what_a_host_holds_back_from_a_round_the_next_round_hands_over() {
    let u = universe(Limits::default());
    let (_, outcome, mut state) = drive(
        RoundMachine::start(RoundKind::Sync, CourierState::default(), u.bob.clone()),
        |request| answer(&u.relay, BOB, false, request),
    );
    assert!(outcome.is_ok());
    let held_event = (WRITE.to_string(), write(ALICE, 1000, 1));
    let held_message = Record::KeyPackage {
        node: BOB,
        data: "AA".to_string(),
        join: None,
    };
    // What became of three sends that waited: one signed, one found held, one refused for good.
    let taken = (WRITE.to_string(), write(BOB, 2000, 2));
    let found = (WRITE.to_string(), write(BOB, 2500, 4));
    let dropped = (
        WRITE.to_string(),
        write(BOB, 3000, 3),
        "node 2 may not write".to_string(),
    );
    // A state that holds no send found held writes no `held`, as before they were told apart.
    assert!(!state.to_text().contains("\"held\""), "{}", state.to_text());
    state.hold(Round {
        events: vec![held_event.clone()],
        messages: vec![held_message.clone()],
        evicted: vec![(ALICE, 9)],
        restored: vec![(ALICE, 8)],
        taken: vec![taken.clone()],
        held: vec![found.clone()],
        dropped: vec![dropped.clone()],
        ..Round::default()
    });
    assert_eq!(state.untold(), 1);
    assert!(state.to_text().contains("\"held\""), "{}", state.to_text());
    let read_back = CourierState::from_text(&state.to_text());
    assert_eq!(read_back.as_ref(), Some(&state), "{}", state.to_text());
    // A text written before sends were kept waiting names none of them, and is read as holding none.
    let mut older: serde_json::Value = serde_json::from_str(&state.to_text()).unwrap();
    let object = older.as_object_mut().unwrap();
    object.remove("taken");
    object.remove("held");
    object.remove("dropped");
    let older = CourierState::from_text(&older.to_string()).expect("an older text is read");
    assert_ne!(older, state, "it holds what became of no send");

    let (_, outcome, after_sync) = drive(
        RoundMachine::start(RoundKind::Sync, state.clone(), u.bob.clone()),
        |request| answer(&u.relay, BOB, false, request),
    );
    let next = outcome.unwrap();
    assert_eq!(next.events, vec![held_event], "nothing else was new");
    assert_eq!(next.messages, vec![held_message]);
    assert_eq!(next.evicted, vec![(ALICE, 9)]);
    assert_eq!(next.restored, vec![(ALICE, 8)]);
    assert_eq!(next.taken, vec![taken.clone()]);
    assert_eq!(next.held, vec![found.clone()]);
    assert_eq!(next.dropped, vec![dropped.clone()]);
    assert_eq!(after_sync.untold(), 0);

    let (_, outcome, _) = drive(
        RoundMachine::start(RoundKind::Resync, state, u.bob.clone()),
        |request| answer(&u.relay, BOB, false, request),
    );
    let resync = outcome.unwrap();
    assert!(resync.messages.is_empty());
    assert_eq!(
        named(&resync, WRITE).len(),
        2,
        "the relay's two writes, once"
    );
    assert_eq!(resync.evicted, vec![(ALICE, 9)]);
    assert_eq!(resync.restored, vec![(ALICE, 8)]);
    assert_eq!(
        (resync.taken, resync.held, resync.dropped),
        (vec![taken], vec![found], vec![dropped]),
        "a resync reads the relay again, and what became of a send is not read again"
    );
}

/// The relay gives two records a batch, so a round has taken alice's writes in when it asks for the
/// batch that holds her note. That answer never comes (the host abandons the round), or the link
/// fails on it; either way the state the round leaves, saved and read back, starts a round that
/// tells bob's machine exactly what an unbroken round told it.
#[test]
fn a_round_whose_answer_never_comes_leaves_the_next_round_to_tell_what_it_took_in() {
    let limits = Limits {
        max_feed: 2,
        ..Limits::default()
    };
    let reference = {
        let unbroken = universe(limits);
        Courier::new(unbroken.bob.clone(), link(&unbroken.relay, BOB).0)
            .sync()
            .unwrap()
    };
    assert_eq!(named(&reference, WRITE).len(), 2);

    for link_fails in [false, true] {
        let broken = universe(limits);
        let mut machine =
            RoundMachine::start(RoundKind::Sync, CourierState::default(), broken.bob.clone());
        let mut feeds = 0;
        loop {
            let Step::Request(request) = machine.poll() else {
                panic!("the round ended before its fourth feed");
            };
            if matches!(request, Request::Feed { .. }) {
                feeds += 1;
                if feeds == 4 {
                    break;
                }
            }
            machine.respond(answer(&broken.relay, BOB, false, &request));
        }
        assert!(machine.is_asking());
        assert!(
            matches!(
                machine.poll(),
                Step::Request(Request::Feed { cursor: 6, .. })
            ),
            "asked again before an answer, it asks the same"
        );
        let state = if link_fails {
            machine.respond(Err(LinkError("the connection closed".to_string())));
            assert!(matches!(
                machine.poll(),
                Step::Done(Err(CourierError::Link(_)))
            ));
            machine.into_state()
        } else {
            let saved = machine.abandoned_state();
            let state = machine.into_state();
            assert_eq!(saved, state, "what is saved mid-round is what is left");
            state
        };
        assert_eq!(
            state.cursor(),
            6,
            "past the batches taken in, and not past the one asked for"
        );
        // The two writes taken in are held by the trust and not told: a round tells its machine
        // what it may when it ends (docs/adr/0085, section 3.1), so one that does not end leaves
        // nothing to hand over, and what the machine was told did not move.
        assert_eq!(state.untold(), 0, "nothing is told before a round ends");
        assert!(
            broken.bob.delivered().is_empty(),
            "the two writes taken in wait in the trust: {:?}",
            broken.bob.delivered()
        );
        let read_back = CourierState::from_text(&state.to_text());
        assert_eq!(read_back.as_ref(), Some(&state), "{}", state.to_text());

        let machine = RoundMachine::start(RoundKind::Sync, read_back.unwrap(), broken.bob.clone());
        let (_, outcome, left) = drive(machine, |request| {
            answer(&broken.relay, BOB, false, request)
        });
        let resumed = outcome.unwrap();
        assert_eq!(resumed.events, reference.events, "link fails: {link_fails}");
        assert_eq!(left.untold(), 0);
        assert_eq!(left.cursor(), 8);
    }
}

/// A courier whose link fails in the middle of a round loses nothing: its next round tells the
/// machine what the broken one took in.
#[test]
fn a_courier_whose_link_fails_mid_round_tells_it_all_in_the_next() {
    struct FailsOnce {
        inner: TestLink,
        feeds: usize,
    }
    impl Link for FailsOnce {
        fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
            if matches!(request, Request::Feed { .. }) {
                self.feeds += 1;
                if self.feeds == 4 {
                    return Err(LinkError("the connection closed".to_string()));
                }
            }
            self.inner.call(request)
        }
    }

    let limits = Limits {
        max_feed: 2,
        ..Limits::default()
    };
    let unbroken = universe(limits);
    let reference = Courier::new(unbroken.bob.clone(), link(&unbroken.relay, BOB).0)
        .sync()
        .unwrap();
    let broken = universe(limits);
    let mut courier = Courier::new(
        broken.bob.clone(),
        FailsOnce {
            inner: link(&broken.relay, BOB).0,
            feeds: 0,
        },
    );
    assert!(matches!(courier.sync(), Err(CourierError::Link(_))));
    assert_eq!(courier.cursor(), 6);
    let resumed = courier.sync().unwrap();
    assert_eq!(resumed.events, reference.events);
}

/// A relay keeps the log back from bob: the round takes alice's first write, finds it names an
/// entry bob lacks, brings the log level once more, gets nothing, and stalls with the cursor before
/// that write. When the log comes, the next round goes on from there and tells everything once. A
/// courier and a machine driven by hand do it alike.
#[test]
fn a_round_that_stalled_on_a_log_kept_back_goes_on_when_the_log_comes() {
    let by_courier = universe(Limits::default());
    let by_hand = universe(Limits::default());
    let withhold = Arc::new(AtomicBool::new(true));
    let asked: Asked = Arc::new(Mutex::new(Vec::new()));
    let mut courier = Courier::new(
        by_courier.bob.clone(),
        TestLink {
            relay: by_courier.relay.clone(),
            node: BOB,
            withhold: withhold.clone(),
            asked: asked.clone(),
        },
    );

    let stalled = courier.sync().unwrap();
    let machine = RoundMachine::start(
        RoundKind::Sync,
        CourierState::default(),
        by_hand.bob.clone(),
    );
    let (requests, outcome, state) = drive(machine, |request| {
        answer(&by_hand.relay, BOB, true, request)
    });
    assert_eq!(requests, taken(&asked));
    assert_eq!(outcome, Ok(stalled.clone()));
    assert!(stalled.stalled);
    assert!(named(&stalled, WRITE).is_empty());
    assert_eq!(
        requests
            .iter()
            .filter(|request| matches!(request, Request::Entries { .. }))
            .count(),
        2,
        "the log was asked for at the start and once more for the write: {requests:?}"
    );
    assert_eq!(state.cursor(), 0);
    assert_eq!(courier.cursor(), 0);
    assert_eq!(by_hand.bob.log_len(), 1);

    withhold.store(false, Ordering::SeqCst);
    let resumed = courier.sync().unwrap();
    let machine = RoundMachine::start(RoundKind::Sync, state, by_hand.bob.clone());
    let (requests, outcome, state) = drive(machine, |request| {
        answer(&by_hand.relay, BOB, false, request)
    });
    assert_eq!(requests, taken(&asked));
    assert_eq!(outcome, Ok(resumed.clone()));
    assert!(!resumed.stalled);
    assert_eq!(named(&resumed, WRITE).len(), 2);
    assert_eq!(named(&resumed, SUPERSEDE).len(), 1);
    assert_eq!(named(&resumed, ROLE).len(), 3);
    assert_eq!(state.cursor(), 8);
}

/// A device the log does not make a member may not read: its round stalls before the feed, and
/// once an administrator grants it, its next round reads everything.
#[test]
fn a_round_of_a_device_that_may_not_read_yet_goes_on_once_it_may() {
    let by_courier = universe(Limits::default());
    let by_hand = universe(Limits::default());
    let dave_by_courier = host_told_the_genesis(DAVE);
    let dave_by_hand = host_told_the_genesis(DAVE);
    let (dave_link, asked) = link(&by_courier.relay, DAVE);
    let mut courier = Courier::new(dave_by_courier, dave_link);

    let stalled = courier.sync().unwrap();
    let machine = RoundMachine::start(
        RoundKind::Sync,
        CourierState::default(),
        dave_by_hand.clone(),
    );
    let (requests, outcome, state) = drive(machine, |request| {
        answer(&by_hand.relay, DAVE, false, request)
    });
    assert_eq!(requests, taken(&asked));
    assert_eq!(outcome, Ok(stalled.clone()));
    assert!(stalled.stalled);
    assert_eq!(stalled.refused.len(), 1, "{:?}", stalled.refused);
    assert!(
        !requests
            .iter()
            .any(|request| matches!(request, Request::Feed { .. })),
        "nothing is read: {requests:?}"
    );

    for each in [&by_courier, &by_hand] {
        let grant = each
            .alice
            .draft(Body::Grant {
                node: DAVE,
                role: 1,
                key: person(DAVE).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        each.alice.append(&grant).unwrap();
        Courier::new(each.alice.clone(), link(&each.relay, ALICE).0)
            .sync()
            .unwrap();
    }
    let resumed = courier.sync().unwrap();
    let machine = RoundMachine::start(RoundKind::Sync, state, dave_by_hand);
    let (requests, outcome, _) = drive(machine, |request| {
        answer(&by_hand.relay, DAVE, false, request)
    });
    assert_eq!(requests, taken(&asked));
    assert_eq!(outcome, Ok(resumed.clone()));
    assert!(!resumed.stalled);
    assert_eq!(named(&resumed, WRITE).len(), 2);
}
