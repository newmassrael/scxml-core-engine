// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A device that held a shared list before a change was one unit (docs/adr/0085, landing 4c) kept a
// sealed state of the core's version 5, whose operations are parts of changes signed under the
// second event domain, and its relay holds the list's history in that domain. The core of this
// decision opens that state and upgrades it (section 8.3): the device keeps its person, its
// administrator's key, its log and its chain's position, and the list it held is the state its
// parts make, which its host gives a machine built again before the history; the next save is of
// the core's own version. Two devices that read the same history from the relay fold the same
// state, and a forged part is refused by the relay and by the core.
//
// Before it, the core did not open a state of version 5, and a relay that held parts did not
// start: an upgraded device showed an empty list.

use std::collections::BTreeMap;

use sce_rust_peer::{Kept, Outcome, Peer, PeerError, Profile, RoundKind};
use sce_rust_relay::{
    Authority, Code, CourierState, Limits, MemoryStore, Record, Relay, Request, Response, Store,
};
use sce_rust_trust::backup::{self, Contents, DataKey};
use sce_rust_trust::membership::{Body, Entry, Log};
use sce_rust_trust::message::{self, Link};
use sce_rust_trust::parts::{self, Dot, Live};
use sce_rust_trust::{encoding, Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

/// The administrator's key, which alice's device holds.
fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![administrator().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator())
    .finish()
}

/// The log of the list: its genesis, alice its owner, and bob and carol granted writing.
fn entries() -> Vec<Entry> {
    let scratch = Trust::open(LIST);
    let genesis = genesis();
    scratch.append(&genesis).unwrap();
    let mut all = vec![genesis];
    for (node, role) in [(ALICE, 3u8), (BOB, 2), (CAROL, 2)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&administrator())
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

/// The parts each device signed, chain by chain, as a device of the second domain signed them.
#[derive(Default)]
struct Chains {
    last: BTreeMap<u64, (u64, [u8; 32])>,
}

impl Chains {
    /// The next part of `author`'s chain, signed with `key`: the event and the data it travels as.
    fn sign_as(
        &mut self,
        author: u64,
        key: &Identity,
        event: &str,
        data: Value,
    ) -> (String, String) {
        let (seq, prev) = self
            .last
            .get(&author)
            .map_or((1, message::NO_OPERATION), |(seq, id)| (seq + 1, *id));
        let log = entries();
        let link = Link {
            seq,
            prev,
            log: log.len() as u64 - 1,
            log_hash: log.last().unwrap().hash(),
            context: Vec::new(),
        };
        let signing = parts::signing(LIST, event, &data).unwrap().unwrap();
        let bytes = parts::operation_bytes(&signing.message, &link);
        let mut wire = data;
        wire["seq"] = json!(seq.to_string());
        wire["prev"] = json!(encoding::encode(&prev));
        wire["mh"] = json!(link.log.to_string());
        wire["mhh"] = json!(encoding::encode(&link.log_hash));
        wire["sig"] = json!(encoding::encode(&key.sign(&bytes)));
        self.last
            .insert(author, (seq, message::operation_id(&bytes)));
        (event.to_string(), wire.to_string())
    }

    fn sign(&mut self, author: u64, event: &str, data: Value) -> (String, String) {
        self.sign_as(author, &person(author), event, data)
    }
}

fn write(node: u64, id: u64, text: &str, wall: i64, of: u64) -> Value {
    json!({"id": id, "field": 3, "value": "0", "text": text, "wallTime": wall.to_string(),
           "counter": 0, "nodeId": node.to_string(), "parts": of})
}

fn write_over(by: u64, id: u64, node: u64, wall: i64, of: u64, at: i64) -> Value {
    let mut part = json!({"id": id, "field": 3, "wallTime": wall.to_string(), "counter": 0,
                          "nodeId": node.to_string(), "by": by.to_string()});
    if of > 1 {
        part["parts"] = json!(of);
        part["atWallTime"] = json!(at.to_string());
        part["atCounter"] = json!(0);
    }
    part
}

/// The list's history in the second domain: a change of two writes, a clear of one value by two
/// devices at once (bob writes over alice's milk with oat milk, carol clears it and alice's eggs),
/// and a write-over sent as a change of one.
fn history() -> Vec<(String, String)> {
    let mut chains = Chains::default();
    let (w, s) = ("record_list.write", "record_list.supersede");
    vec![
        chains.sign(ALICE, w, write(ALICE, 1, "milk", 1000, 1)),
        chains.sign(ALICE, w, write(ALICE, 2, "eggs", 1100, 2)),
        chains.sign(ALICE, w, write(ALICE, 4, "today", 1100, 2)),
        chains.sign(BOB, s, write_over(BOB, 1, ALICE, 1000, 2, 1200)),
        chains.sign(BOB, w, write(BOB, 1, "oat milk", 1200, 2)),
        chains.sign(CAROL, s, write_over(CAROL, 1, ALICE, 1000, 2, 1250)),
        chains.sign(CAROL, s, write_over(CAROL, 2, ALICE, 1100, 2, 1250)),
        chains.sign(ALICE, s, write_over(ALICE, 4, ALICE, 1100, 1, 0)),
        chains.sign(ALICE, w, write(ALICE, 3, "bread", 1400, 1)),
    ]
}

/// The rows the list held: bob's oat milk and alice's bread.
fn the_rows() -> Vec<Live> {
    let live = |id, node, wall: i64, text: &str| Live {
        id,
        field: 3,
        dot: Dot {
            wall_time: wall,
            counter: 0,
            node,
        },
        value: 0,
        text: text.to_string(),
    };
    vec![
        live(1, BOB, 1200, "oat milk"),
        live(3, ALICE, 1400, "bread"),
    ]
}

/// A relay that held the list before a change was one unit: its log and its history of parts.
struct Network {
    relay: Relay<MemoryStore>,
    authority: Authority,
}

impl Network {
    fn holding(operations: &[(String, String)]) -> Self {
        let authority = Authority::new([3; 32], "the-relay");
        let relay = Relay::open(store_of(operations), authority.clone(), Limits::default())
            .expect("a relay that holds parts starts");
        Self { relay, authority }
    }

    fn carry(&mut self, node: u64, request: &str) -> String {
        let token = self.authority.issue(LIST, node, u64::MAX);
        self.relay.handle(&token, request, 0)
    }

    /// A round of `peer`, every request carried to the relay and every answer back.
    fn round(&mut self, peer: &mut Peer, node: u64) -> Outcome {
        peer.start(RoundKind::Sync).unwrap();
        while let Some(request) = peer.poll_request() {
            let response = self.carry(node, &request);
            peer.respond(&response).unwrap();
        }
        let outcome = peer
            .take_outcome()
            .expect("the round ended")
            .expect("the round held");
        assert!(outcome.refused().is_empty(), "{:?}", outcome.refused());
        outcome
    }
}

/// A store holding the log and `operations`, as a relay kept them.
fn store_of(operations: &[(String, String)]) -> MemoryStore {
    let mut store = MemoryStore::new();
    for entry in entries() {
        store.append(LIST, &Record::Entry(entry.to_json())).unwrap();
    }
    for (event, data) in operations {
        let record = Record::Operation {
            event: event.clone(),
            data: data.clone(),
        };
        store.append(LIST, &record).unwrap();
    }
    store
}

/// Alice's device as a core of version 5 sealed it: her person, the administrator's key, the log,
/// every part she held, and what the relay's client kept, under `key`.
fn sealed_at_version_5(key: &DataKey) -> Vec<u8> {
    let trust = json!({
        "checkpoints": [],
        "operations": history().iter().map(|(event, data)| json!([event, data])).collect::<Vec<_>>(),
        "waiting": [],
    });
    let state = json!({
        "version": 5,
        "log": entries().iter().map(Entry::to_json).collect::<Vec<_>>(),
        "trust": trust.to_string(),
        "courier": CourierState::default().to_text(),
        "owed": [],
        "invitations": [],
        "joins": [],
    });
    let contents = Contents::of_identity(LIST, ALICE, &person(ALICE))
        .with("peer/state", state.to_string().into_bytes())
        .with_identity("peer/administrator", &administrator());
    backup::seal_state(key, "sce-peer/state/5", &contents).expect("the state seals")
}

fn invited(node: u64) -> Peer {
    let mut peer = Peer::invited(LIST, genesis().hash());
    peer.set_identity(node, person(node));
    peer
}

/// A state of version 5 opens under its key and is upgraded: the device keeps its person, its
/// administrator's key and its log, holds every part where it stands in its chain, and its list is
/// the rows its parts make; it waits for its own chain as every restored device does, signs its
/// next change at the number after its last part, and its next save is of the core's version,
/// which opens again with the same rows. Red if the state of version 5 is refused, if a part is
/// dropped as unsigned, or if the change after it is signed at another number.
#[test]
fn a_device_saved_at_version_5_opens_with_the_rows_it_held() {
    let key = DataKey::from_bytes([4; 32]);
    let sealed = sealed_at_version_5(&key);
    let mut alice = Peer::invited(LIST, genesis().hash());
    let kept = alice
        .restore_sealed(ALICE, &sealed, &key)
        .expect("a state of version 5 opens");
    assert_eq!(kept, history().len(), "every part is held");

    let standing = alice.standing();
    assert_eq!(standing.person, Some((ALICE, person(ALICE).public())));
    assert!(standing.administrator, "the administrator's key is kept");
    assert_eq!(standing.role, 3);
    assert!(standing.awaits_own_chain);
    assert_eq!(
        alice.trust().last_seq(ALICE),
        5,
        "the chain's position is kept"
    );

    let fold = alice.fold();
    assert_eq!(fold.values, the_rows());
    assert_eq!(fold.cut, vec![(ALICE, 5), (BOB, 2), (CAROL, 2)]);
    let restores = fold.restores(64);
    assert_eq!(restores.len(), 1);
    let restore: Value = serde_json::from_str(&restores[0]).unwrap();
    assert_eq!(restore["wallTime"], json!("1400"));
    assert_eq!(restore["ops"][0]["text"], json!("oat milk"));
    assert_eq!(restore["ops"][0]["nodeId"], json!("2"));
    assert!(
        alice.history(&[]).changes.is_empty(),
        "no machine is told a part: the list is the fold"
    );

    let mut network = Network::holding(&history());
    let first = network.round(&mut alice, ALICE);
    assert!(!first.rebuild(), "the parts read again were held");
    assert!(!alice.standing().awaits_own_chain);
    let change = json!({"by": "1", "wallTime": "1500", "counter": 0, "ops": [
        {"kind": 0, "id": 5, "field": 3, "value": "0", "text": "jam",
         "wallTime": "0", "counter": 0, "nodeId": "0"}], "command": "1"})
    .to_string();
    assert_eq!(alice.made(&change), Kept::Taken);
    let records = alice.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].seq, 6, "the change follows the last part");

    let saved = alice.save_sealed().expect("the upgraded device saves");
    let mut again = Peer::invited(LIST, genesis().hash());
    again
        .restore_sealed(ALICE, &saved.state, &saved.key)
        .expect("the next save is of the core's own version");
    assert_eq!(again.fold(), alice.fold());
    assert_eq!(again.trust().last_seq(ALICE), 6);
}

/// Two devices that read the same history of parts from a relay fold the same rows, and are told
/// to build their machines again for them; a change made after the fold reaches each in its round.
/// Red if a device folds what it read in the order it read it, or tells its machine a part.
#[test]
fn two_devices_that_fold_the_same_relay_history_agree() {
    let mut network = Network::holding(&history());
    let mut bob = invited(BOB);
    let mut carol = invited(CAROL);
    let bobs = network.round(&mut bob, BOB);
    let carols = network.round(&mut carol, CAROL);
    assert!(bobs.rebuild() && carols.rebuild(), "the fold moved");
    assert!(bobs
        .events()
        .iter()
        .all(|(event, _)| event == "record_list.role"));
    assert_eq!(bob.fold(), carol.fold());
    assert_eq!(bob.fold().values, the_rows());

    let change = json!({"by": "3", "wallTime": "1500", "counter": 0, "ops": [
        {"kind": 1, "id": 3, "field": 3, "value": "0", "text": "",
         "wallTime": "1400", "counter": 0, "nodeId": "1"}], "command": "1"})
    .to_string();
    assert_eq!(carol.made(&change), Kept::Taken);
    let records = carol.take_records();
    assert_eq!(records[0].seq, 3, "carol's change follows her parts");
    carol.recorded(records[0].seq);
    network.round(&mut carol, CAROL);
    let told = network.round(&mut bob, BOB);
    let events = told.events();
    let changes: Vec<&(String, String)> = events
        .iter()
        .filter(|(event, _)| event == "record_list.change")
        .collect();
    assert_eq!(changes.len(), 1, "{events:?}");
    assert!(!told.rebuild(), "no part came");
    assert_eq!(
        bob.fold(),
        carol.fold(),
        "the fold is the parts', not the change's"
    );
}

/// A forged part is refused: a relay given one by a session refuses it and keeps nothing, a relay
/// whose store holds one does not start, and a core given one in a round's feed never gets it,
/// since no relay holds it. Red if a part's signature is not checked by the relay or the core.
#[test]
fn a_forged_part_is_refused_by_the_relay_and_the_core() {
    let mut chains = Chains::default();
    let forged = chains.sign_as(
        ALICE,
        &person(BOB),
        "record_list.write",
        write(ALICE, 1, "beer", 1000, 1),
    );
    let mut network = Network::holding(&[]);
    let submitted = network.carry(
        BOB,
        &Request::Submit {
            event: forged.0.clone(),
            data: forged.1.clone(),
        }
        .to_json(),
    );
    match Response::from_json(&submitted) {
        Ok(Response::Refused { code, reason }) => {
            assert_eq!(code, Code::Invalid);
            assert!(reason.contains("the signature is not node 1's"), "{reason}");
        }
        other => panic!("a forged part is refused: {other:?}"),
    }
    assert!(
        Relay::open(
            store_of(&[forged.clone()]),
            Authority::new([3; 32], "the-relay"),
            Limits::default()
        )
        .is_err(),
        "a relay whose store holds a forged part does not start"
    );

    let mut bob = invited(BOB);
    network.round(&mut bob, BOB);
    assert!(bob.fold().values.is_empty(), "nothing forged was held");
    assert!(matches!(
        bob.trust().take_operation(&forged.0, &forged.1),
        Err(sce_rust_trust::TakeError::Refused(_))
    ));
    assert!(matches!(
        Peer::invited(LIST, genesis().hash()).restore_sealed(
            ALICE,
            b"not a sealed state",
            &DataKey::from_bytes([4; 32])
        ),
        Err(PeerError::Unreadable(_))
    ));
}
