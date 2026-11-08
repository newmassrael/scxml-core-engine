// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A relay may give a host the operations of a list in any order and may leave one out of its feed:
// it reads none of them, and the order it keeps is the order they reached it. A host's round tells
// its machine an operation only after every operation it follows (its device's earlier ones, and
// the ones its context names), and asks the relay for each one it lacks by its device and number,
// so a write the feed withheld is repaired and told before the write-over that names it, never
// skipped (docs/adr/0085, section 3).
//
// Before the rule, the round told the machine each operation as the feed gave it: the write-over
// alone, and the write never, which is the order that brings a written-over value back once the
// list keeps no tombstones.

use std::sync::{Arc, Mutex};

use sce_rust_relay::{
    Authority, Courier, Limits, Link, LinkError, MemoryStore, Record, Relay, Request, Response,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const WRITE: &str = "record_list.write";
const SUPERSEDE: &str = "record_list.supersede";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

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
    for node in [ALICE, BOB, CAROL] {
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

fn authority() -> Authority {
    Authority::new([7; 32], "relay-a")
}

fn token(node: u64) -> String {
    authority().issue(LIST, node, 1_000_000)
}

type Shared = Arc<Mutex<Relay<MemoryStore>>>;

/// A link to the relay in this process for `node`, which leaves out of every feed the operations
/// `withheld` says to, and answers every other request as the relay does.
struct RelayLink {
    relay: Shared,
    node: u64,
    withheld: fn(&Record) -> bool,
    asked: Arc<Mutex<Vec<Request>>>,
}

impl Link for RelayLink {
    fn call(&mut self, request: &Request) -> Result<Response, LinkError> {
        self.asked.lock().unwrap().push(request.clone());
        let text = self
            .relay
            .lock()
            .unwrap()
            .handle(&token(self.node), &request.to_json(), 0);
        let response = Response::from_json(&text).map_err(LinkError)?;
        Ok(match response {
            Response::Feed {
                records,
                next,
                more,
            } => Response::Feed {
                records: records
                    .into_iter()
                    .filter(|record| !(self.withheld)(record))
                    .collect(),
                next,
                more,
            },
            other => other,
        })
    }
}

fn nothing_withheld(_: &Record) -> bool {
    false
}

/// Alice's write, which the feed carol reads leaves out.
fn alices_write(record: &Record) -> bool {
    matches!(record, Record::Operation { event, data }
        if event == WRITE && data.contains("\"nodeId\":\"1\""))
}

fn courier(
    relay: &Shared,
    host: &Trust,
    node: u64,
    withheld: fn(&Record) -> bool,
) -> Courier<RelayLink> {
    courier_asking(relay, host, node, withheld).0
}

fn courier_asking(
    relay: &Shared,
    host: &Trust,
    node: u64,
    withheld: fn(&Record) -> bool,
) -> (Courier<RelayLink>, Arc<Mutex<Vec<Request>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let link = RelayLink {
        relay: relay.clone(),
        node,
        withheld,
        asked: asked.clone(),
    };
    (Courier::new(host.clone(), link), asked)
}

/// The operations a round told the machine, each by its event and the value of `wallTime`, and of
/// `by` for a write-over: the roster is left out.
fn operations_told(events: &[(String, String)]) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|(event, _)| event == WRITE || event == SUPERSEDE)
        .map(|(event, data)| {
            let data: Value = serde_json::from_str(data).unwrap();
            let by = data["by"]
                .as_str()
                .map_or(String::new(), |by| format!("/{by}"));
            let wall = data["wallTime"].as_str().unwrap_or("").to_string();
            (event.clone(), format!("{wall}{by}"))
        })
        .collect()
}

fn event(event: &str, what: &str) -> (String, String) {
    (event.to_string(), what.to_string())
}

#[test]
fn a_write_the_feed_withheld_is_asked_for_and_told_before_the_write_over_that_names_it() {
    let relay: Shared = Arc::new(Mutex::new(
        Relay::open(
            MemoryStore::new(),
            authority(),
            Limits {
                burst: 1_000_000,
                refill_per_second: 1_000_000,
                ..Limits::default()
            },
        )
        .unwrap(),
    ));
    let alice = Trust::open(LIST);
    for entry in entries() {
        alice.append(&entry).unwrap();
    }
    alice.set_identity(ALICE, person(ALICE));
    let write = json!({"id": 1, "field": 3, "value": "1", "text": "",
                       "wallTime": "1000", "counter": 0, "nodeId": "1"})
    .to_string();
    assert!(alice.seal_sent(WRITE, &write).unwrap().unwrap().signed);
    let round = courier(&relay, &alice, ALICE, nothing_withheld)
        .sync()
        .unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);

    // Bob's machine is told alice's write, and bob writes over it.
    let bob = Trust::open(LIST);
    bob.append(&entries()[0]).unwrap();
    bob.set_identity(BOB, person(BOB));
    let mut bobs = courier(&relay, &bob, BOB, nothing_withheld);
    let round = bobs.sync().unwrap();
    assert_eq!(operations_told(&round.events), vec![event(WRITE, "1000")]);
    let note = json!({"id": 1, "field": 3, "wallTime": "1000", "counter": 0,
                      "nodeId": "1", "by": "2"})
    .to_string();
    assert!(bob.seal_sent(SUPERSEDE, &note).unwrap().unwrap().signed);
    let round = bobs.sync().unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);

    // Carol's feed leaves alice's write out: her round asks for it by its device and number, and
    // tells her machine the write, then the write-over.
    let carol = Trust::open(LIST);
    carol.append(&entries()[0]).unwrap();
    carol.set_identity(CAROL, person(CAROL));
    let (mut carols, asked) = courier_asking(&relay, &carol, CAROL, alices_write);
    let round = carols.sync().unwrap();
    assert!(
        asked.lock().unwrap().contains(&Request::Operations {
            node: ALICE,
            from: 1,
            to: 1
        }),
        "the round asks for the write its write-over follows: {:?}",
        asked.lock().unwrap()
    );
    assert_eq!(
        operations_told(&round.events),
        vec![event(WRITE, "1000"), event(SUPERSEDE, "1000/2")]
    );
    assert_eq!(carol.delivered(), vec![(ALICE, 1), (BOB, 1)]);
    assert!(carol.lacking().is_empty());

    // The next round reads nothing new and tells nothing again.
    let round = carols.sync().unwrap();
    assert!(operations_told(&round.events).is_empty());
}
