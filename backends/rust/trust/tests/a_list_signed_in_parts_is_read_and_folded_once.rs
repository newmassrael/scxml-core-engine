// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A list a relay or a device held before a change was one unit (docs/adr/0085, landing 4c) holds
// its changes as parts: each operation of a change a signed event of its own, `record_list.write`
// or `record_list.supersede`, under the second event domain and the first operation domain, with
// no causal context. A host of this decision reads them and writes none (section 8.3): each part is
// verified as it was signed, and a forged one is refused; it is held where it stands in its
// author's chain, so the device's next change follows its last part; no machine is told one; and
// what the parts hold together is folded into one state, the same whatever order a host took them
// in (section 9.5), which a machine built again is given first. A part signed after a change of its
// device is held as proof, as a fork is. What a device's machine sent as parts and its person had
// not signed waits again as the changes they were parts of.
//
// The bytes the reader checks a part against are held to the vectors the second domain's own
// encoder pinned before it was removed (`a_byte_format_does_not_drift` before landing 4c), so the
// reader is the old encoder's and not one that only agrees with itself.

use std::collections::BTreeMap;

use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link};
use sce_rust_trust::parts::{self, Dot, Live};
use sce_rust_trust::{encoding, secrecy, Identity, Sent, TakeError, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE_EVENT: &str = "record_list.change";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

/// The log of the list: its genesis, and alice, bob and carol granted writing.
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

/// A host holding the whole log, whose person is `node` when one is given.
fn host(node: Option<u64>) -> Trust {
    let host = Trust::open(LIST);
    for entry in entries() {
        host.append(&entry).unwrap();
    }
    if let Some(node) = node {
        host.set_identity(node, person(node));
    }
    host
}

/// The parts each device signed, chain by chain, as a device of the second domain signed them:
/// the part's bytes under the second event domain, its link (no context) under the first operation
/// domain, and its author's signature of both, written into the data as a part travelled.
#[derive(Default)]
struct Chains {
    last: BTreeMap<u64, (u64, [u8; 32])>,
}

impl Chains {
    /// The next part of `author`'s chain: the event and the data it travels as.
    fn sign(&mut self, author: u64, event: &str, data: Value) -> (String, String) {
        self.sign_as(author, &person(author), event, data)
    }

    /// The next part of `author`'s chain, signed with `key`.
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
        let (wire, id) = part_at(key, event, data, seq, prev);
        self.last.insert(author, (seq, id));
        (event.to_string(), wire)
    }
}

/// `data` as the part `event` at `seq` after `prev`, signed with `key` under the last entry of the
/// log: the data it travels as, and its id.
fn part_at(
    key: &Identity,
    event: &str,
    data: Value,
    seq: u64,
    prev: [u8; 32],
) -> (String, [u8; 32]) {
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
    wire["seq"] = json!(link.seq.to_string());
    wire["prev"] = json!(encoding::encode(&link.prev));
    wire["mh"] = json!(link.log.to_string());
    wire["mhh"] = json!(encoding::encode(&link.log_hash));
    wire["sig"] = json!(encoding::encode(&key.sign(&bytes)));
    (wire.to_string(), message::operation_id(&bytes))
}

/// A write of `node`'s, alone or as one of `parts` of its change, at `wall`.
fn write(node: u64, id: u64, field: u64, text: &str, wall: i64, of: u64) -> Value {
    json!({"id": id, "field": field, "value": "0", "text": text, "wallTime": wall.to_string(),
           "counter": 0, "nodeId": node.to_string(), "parts": of})
}

/// A write-over by `by` of the value `node` wrote at `wall`, one of `of` parts of `by`'s change
/// at `at`; a change of one carries no stamp of its own.
fn write_over(by: u64, id: u64, field: u64, node: u64, wall: i64, of: u64, at: i64) -> Value {
    let mut part = json!({"id": id, "field": field, "wallTime": wall.to_string(), "counter": 0,
                          "nodeId": node.to_string(), "by": by.to_string()});
    if of > 1 {
        part["parts"] = json!(of);
        part["atWallTime"] = json!(at.to_string());
        part["atCounter"] = json!(0);
    }
    part
}

/// A relay's history of the second domain: changes of one part and of more, and two concurrent
/// clears of one value (bob and carol each write over the milk alice wrote, neither having seen
/// the other's).
fn history() -> Vec<(String, String)> {
    let mut chains = Chains::default();
    let (w, s) = ("record_list.write", "record_list.supersede");
    vec![
        chains.sign(ALICE, w, write(ALICE, 1, 3, "milk", 1000, 1)),
        chains.sign(ALICE, w, write(ALICE, 2, 3, "eggs", 1100, 2)),
        chains.sign(ALICE, w, write(ALICE, 2, 4, "today", 1100, 2)),
        chains.sign(BOB, s, write_over(BOB, 1, 3, ALICE, 1000, 2, 1200)),
        chains.sign(BOB, w, write(BOB, 1, 3, "oat milk", 1200, 2)),
        chains.sign(CAROL, s, write_over(CAROL, 1, 3, ALICE, 1000, 2, 1250)),
        chains.sign(CAROL, s, write_over(CAROL, 2, 3, ALICE, 1100, 2, 1250)),
        chains.sign(ALICE, s, write_over(ALICE, 2, 4, ALICE, 1100, 1, 0)),
        chains.sign(ALICE, w, write(ALICE, 3, 3, "bread", 1400, 1)),
    ]
}

/// The state the history makes: what bob wrote over the milk, and alice's bread.
fn the_state() -> Vec<Live> {
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

fn take_all(host: &Trust, operations: &[(String, String)]) {
    for (event, data) in operations {
        let taken = host
            .take_operation(event, data)
            .unwrap_or_else(|error| panic!("{event} {data}: {error:?}"));
        assert!(taken.new && taken.applied, "{event} {data}");
    }
}

/// The reader gives the bytes the second domain's own encoder gave: a write, a write-over alone
/// and one of a change of two, the operation a part was made of and its id, the signature of it,
/// and the content sealed under a write's stamp and the first content domain. The vectors are
/// the ones that encoder pinned. Red if the reader writes a field otherwise, or in another order,
/// or names another domain.
#[test]
fn the_reader_checks_a_part_against_the_bytes_it_was_signed_as() {
    let write =
        json!({"id":1,"field":3,"value":"10","text":"","wallTime":"1000","counter":0,"nodeId":"1"});
    let note = json!({"id":1,"field":3,"wallTime":"1000","counter":0,"nodeId":"2","by":"1"});
    let signing = parts::signing("home", "record_list.write", &write)
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&signing.message),
        "00000000000000117363652d74727573742f6576656e742f320000000000000004686f6d6500000000000000117265636f72645f6c6973742e7772697465000000000000000131000000000000000133000000000000000231300000000000000000000000000000000431303030000000000000000130000000000000000131000000000000000131"
    );
    assert_eq!(
        signing.author, 1,
        "a write is signed by the node that stamped it"
    );
    let link = Link {
        seq: 3,
        prev: [7; 32],
        log: 4,
        log_hash: [9; 32],
        context: Vec::new(),
    };
    let operation = parts::operation_bytes(&signing.message, &link);
    assert_eq!(
        hex(&operation),
        "00000000000000157363652d74727573742f6f7065726174696f6e2f31000000000000008900000000000000117363652d74727573742f6576656e742f320000000000000004686f6d6500000000000000117265636f72645f6c6973742e7772697465000000000000000131000000000000000133000000000000000231300000000000000000000000000000000431303030000000000000000130000000000000000131000000000000000131000000000000000133000000000000002b42776348427763484277634842776348427763484277634842776348427763484277634842776348427763000000000000000134000000000000002b43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b4a43516b"
    );
    assert_eq!(
        encoding::encode(&message::operation_id(&operation)),
        "JInVDbQHgBWrzZxfJ9gmcgelmJ9Abr2ZLLVzY5bzdIY"
    );
    let signature: [u8; 64] = encoding::decode(
        "86eN9Yh6yXI0W7KQCvMTNO-RBIA2lyaA6SNT3QYpGeQt5ZwfytNESWz8zL3WdVyMAZQR7L3_osDEYkD0xe7fAA",
    )
    .unwrap()
    .try_into()
    .unwrap();
    assert!(person(1).public().verifies(&operation, &signature));

    assert_eq!(
        hex(&parts::signing("home", "record_list.supersede", &note)
            .unwrap()
            .unwrap()
            .message),
        "00000000000000117363652d74727573742f6576656e742f320000000000000004686f6d6500000000000000157265636f72645f6c6973742e737570657273656465000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000132000000000000000131000000000000000131000000000000000130000000000000000130"
    );
    let mut of_two = note.clone();
    of_two["parts"] = json!(2);
    of_two["atWallTime"] = json!("2000");
    of_two["atCounter"] = json!(1);
    let of_two = parts::signing("home", "record_list.supersede", &of_two)
        .unwrap()
        .unwrap();
    assert_eq!(
        hex(&of_two.message),
        "00000000000000117363652d74727573742f6576656e742f320000000000000004686f6d6500000000000000157265636f72645f6c6973742e737570657273656465000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000132000000000000000131000000000000000132000000000000000432303030000000000000000131"
    );
    assert_eq!(of_two.author, 1, "a write-over is signed by who wrote over");

    let stamp = parts::write_stamp(&write).unwrap();
    assert_eq!(
        hex(&stamp),
        "000000000000000131000000000000000133000000000000000431303030000000000000000130000000000000000131"
    );
    let keys = BTreeMap::from([(2u32, [7u8; 32])]);
    let sealed =
        "~AAAAAnNxtIUL1OyQihxj0hPpffYTNYe5LDXx-J_TiFDSbvPeQjjxUObsr9p9qncyyKg541lfLarzNyxFifo";
    assert_eq!(
        secrecy::open_part("home", &keys, &stamp, sealed),
        Ok((-42, "buy milk".to_string()))
    );
    assert_eq!(
        secrecy::open("home", &keys, &stamp, sealed),
        Err(secrecy::OpenError::Corrupt),
        "a write of a change is sealed under another domain"
    );
}

/// Every order a host takes the same parts in folds one state: the live values are the writes no
/// write-over names, whichever came first, so a value two concurrent changes cleared is gone and
/// the write that took its place is live (docs/adr/0085, section 9.5). No machine is told a part.
/// Red if the fold reads the parts in an order and lets a write read after its write-over come
/// back, or if a part is told to a machine.
#[test]
fn two_hosts_that_hold_the_same_parts_fold_one_state_whatever_the_order() {
    let history = history();
    let mut orders: Vec<Vec<(String, String)>> = vec![history.clone()];
    orders.push(history.iter().rev().cloned().collect());
    let mut by_author = history.clone();
    by_author.sort_by_key(|(_, data)| {
        let value: Value = serde_json::from_str(data).unwrap();
        let author = value
            .get("by")
            .or_else(|| value.get("nodeId"))
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        std::cmp::Reverse(author)
    });
    orders.push(by_author);

    let mut folds = Vec::new();
    for order in &orders {
        let reader = host(Some(BOB));
        take_all(&reader, order);
        assert!(
            reader.deliver().is_empty(),
            "a part is told to no machine one by one"
        );
        folds.push(reader.fold());
    }
    for fold in &folds[1..] {
        assert_eq!(fold, &folds[0]);
    }
    let fold = &folds[0];
    assert_eq!(fold.values, the_state());
    assert_eq!((fold.wall_time, fold.counter), (1400, 0));
    assert_eq!(fold.cut, vec![(ALICE, 5), (BOB, 2), (CAROL, 2)]);

    // A relay that holds no key and no person folds the same, and gives every part as it was
    // signed, with no context: a host given them from it holds what a host given them first did.
    let relay = host(None);
    take_all(&relay, &history);
    let reader = host(Some(CAROL));
    take_all(&reader, &relay.signed_operations());
    assert_eq!(reader.fold(), folds[0]);
    for (_, data) in relay.signed_operations() {
        assert!(
            !data.contains("\"cx\""),
            "a part travels without a context: {data}"
        );
    }
}

/// A forged part is refused as any forged operation is: one its author's key did not sign, one
/// changed after it was signed, and one that carries a context its author never signed. Nothing of
/// them is held or folded. Red if a part's signature is not checked, or is checked over bytes that
/// leave a field out.
#[test]
fn a_forged_part_is_refused_and_folds_nothing() {
    let reader = host(Some(BOB));
    let mut chains = Chains::default();
    let (event, forged) = chains.sign_as(
        ALICE,
        &person(CAROL),
        "record_list.write",
        write(ALICE, 1, 3, "milk", 1000, 1),
    );
    match reader.take_operation(&event, &forged) {
        Err(TakeError::Refused(reason)) => {
            assert!(reason.contains("the signature is not node 1's"), "{reason}")
        }
        other => panic!("a part alice's key did not sign is refused: {other:?}"),
    }

    let mut chains = Chains::default();
    let (event, honest) = chains.sign(
        ALICE,
        "record_list.write",
        write(ALICE, 1, 3, "milk", 1000, 1),
    );
    let mut changed: Value = serde_json::from_str(&honest).unwrap();
    changed["text"] = json!("beer");
    assert!(matches!(
        reader.take_operation(&event, &changed.to_string()),
        Err(TakeError::Refused(_))
    ));
    let mut with_context: Value = serde_json::from_str(&honest).unwrap();
    with_context["cx"] = json!([]);
    assert!(matches!(
        reader.take_operation(&event, &with_context.to_string()),
        Err(TakeError::Refused(_))
    ));
    assert_eq!(reader.last_seq(ALICE), 0, "nothing of alice is held");
    assert!(reader.fold().values.is_empty());

    assert!(
        reader.take_operation(&event, &honest).is_ok(),
        "the part as it was signed holds"
    );
    assert_eq!(reader.fold().values.len(), 1);
}

/// The trust signs no part: a part is read and never written (docs/adr/0085, section 8.1). A
/// device whose chain holds its parts signs its next change at the number after its last part,
/// naming it; a part it signed after that change is held as proof, from the part's own number, so
/// the change before it is still told. Red if a part is signed, if the change does not follow the
/// parts, or if a part past a change is taken as a link of the chain.
#[test]
fn a_device_signs_changes_after_its_parts_and_a_part_after_a_change_is_proof() {
    let alice = host(Some(ALICE));
    let history = history();
    take_all(&alice, &history);
    let part = write(ALICE, 4, 3, "jam", 1500, 1).to_string();
    assert_eq!(alice.keep_sent("record_list.write", &part), Sent::NotSigned);
    assert_eq!(alice.seal_sent("record_list.supersede", &part), Ok(None));
    assert_eq!(alice.last_seq(ALICE), 5);

    alice.deliver();
    let change = json!({"by": "1", "wallTime": "1500", "counter": 0, "ops": [
        {"kind": 0, "id": 4, "field": 3, "value": "0", "text": "jam",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string();
    let sealed = alice
        .seal_sent(CHANGE_EVENT, &change)
        .unwrap()
        .expect("a change is signed");
    assert!(sealed.signed);
    assert_eq!(alice.last_seq(ALICE), 6, "the change follows the last part");
    let mut travelling: Value = serde_json::from_str(&sealed.data).unwrap();
    travelling.as_object_mut().unwrap().remove("sig");
    let link = Link::take_from(&mut travelling).unwrap();
    assert_eq!(link.seq, 6);
    let signing = message::signing(LIST, CHANGE_EVENT, &travelling)
        .unwrap()
        .unwrap();
    let change_id = message::operation_id(&message::operation_bytes(&signing.message, &link));

    // Alice's key signs a part after her change, as an old core of hers would.
    let (event, late) = {
        let (wire, _) = part_at(
            &person(ALICE),
            "record_list.write",
            write(ALICE, 5, 3, "tea", 1600, 1),
            7,
            change_id,
        );
        ("record_list.write".to_string(), wire)
    };
    let bob = host(Some(BOB));
    take_all(&bob, &history);
    bob.take_operation(CHANGE_EVENT, &sealed.data).unwrap();
    let taken = bob.take_operation(&event, &late).unwrap();
    assert!(taken.new && !taken.applied, "held as proof");
    assert_eq!(
        taken.conflict.map(|conflict| conflict.after),
        Some(7),
        "frozen from the part's own number"
    );
    assert_eq!(bob.equivocating(), vec![(ALICE, 7)]);
    let told = bob.deliver();
    assert_eq!(
        told.len(),
        1,
        "the change before the part is told: {told:?}"
    );
    assert_eq!(told[0].0, CHANGE_EVENT);
    assert_eq!(
        bob.fold().values,
        the_state(),
        "a part held as proof is folded by no one"
    );
}

/// A text a host wrote before a change was one unit holds parts, verified and held again; what its
/// machine sent that its person had not signed is parts too, which wait again as the changes they
/// were parts of, recalled as provisional and signed by the next signing of what waits; a
/// write-over sent as a change of one, whose change's stamp was never sent, is withdrawn with every
/// send after it, and said so once. A text written again holds the parts as they were signed. Red
/// if the waiting parts are dropped, or carried as one change each.
#[test]
fn a_text_written_before_a_change_was_one_unit_is_read_whole() {
    let history = history();
    let old_text = json!({
        "checkpoints": [],
        "operations": history.iter().map(|(event, data)| json!([event, data])).collect::<Vec<_>>(),
        "waiting": [
            ["record_list.supersede", write_over(ALICE, 3, 3, ALICE, 1400, 2, 1500).to_string()],
            ["record_list.write", write(ALICE, 3, 3, "rye bread", 1500, 2).to_string()],
            ["record_list.write", write(ALICE, 4, 3, "jam", 1510, 1).to_string()],
            ["record_list.supersede", write_over(ALICE, 4, 3, ALICE, 1510, 1, 0).to_string()],
            ["record_list.write", write(ALICE, 5, 3, "tea", 1520, 1).to_string()],
        ],
    })
    .to_string();
    let alice = host(Some(ALICE));
    assert_eq!(alice.restore_from_text(&old_text), history.len());
    assert_eq!(alice.last_seq(ALICE), 5);
    assert_eq!(alice.fold().values, the_state());
    assert_eq!(alice.waiting(), 2, "two changes carried");

    let recalled = alice.history(&[]);
    let provisional: Vec<Value> = recalled
        .iter()
        .filter(|told| told.provisional())
        .map(|told| {
            assert_eq!(told.event, CHANGE_EVENT);
            serde_json::from_str(&told.data).unwrap()
        })
        .collect();
    assert_eq!(provisional.len(), 2);
    assert_eq!(provisional[0]["wallTime"], json!("1500"));
    assert_eq!(provisional[0]["ops"].as_array().unwrap().len(), 2);
    assert_eq!(provisional[0]["ops"][0]["kind"], json!(1));
    assert_eq!(provisional[1]["wallTime"], json!("1510"));

    let written_again = host(Some(ALICE));
    assert_eq!(
        written_again.restore_from_text(&alice.to_text()),
        history.len()
    );
    assert_eq!(written_again.fold(), alice.fold());
    assert_eq!(written_again.waiting(), 2);

    let waited = alice.sign_waiting();
    assert_eq!(waited.taken.len(), 2, "{waited:?}");
    assert_eq!(alice.last_seq(ALICE), 7);
    let withdrawn: Vec<&str> = waited
        .dropped
        .iter()
        .map(|(event, _, _)| event.as_str())
        .collect();
    assert_eq!(withdrawn, ["record_list.supersede", "record_list.write"]);
    assert!(alice.sign_waiting().dropped.is_empty(), "said once");
}
