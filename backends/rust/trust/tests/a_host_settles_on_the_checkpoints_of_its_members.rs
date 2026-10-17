// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An instant is settled on the word of the members, and no administrator's. Each device that is a
// writer signs a checkpoint: the instant, the digest its copy holds of everything up to it, and the
// head of every device's chain it holds in full. The checkpoints of every writer, all for one
// instant and one digest, are a settlement that a host which holds the log checks alone; a hub holds
// none of the devices' keys and cannot make one, and a settlement from fewer devices than the
// writers, from a reader, or for another digest does not hold. The same heads show a relay that
// withholds an operation, to the members who were given it, and the operation is fetched from any
// other host that holds it and checked as it would be from anyone.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Checkpoint, CheckpointError, Divergence, Identity, Settlement, Trust};
use serde_json::{json, Value};

const CALENDAR: &str = "home";
const FLOOR: &str = r#"{"before":"30000","hash":"7"}"#;

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

/// The entries of a log of `list`: the genesis and a grant for each of `grants`, as `(node, role)`
/// where a node's key is the one made from its own number.
fn entries_of(list: &str, grants: &[(u64, u8)]) -> Vec<Entry> {
    let scratch = Trust::open(list);
    let genesis = Log::genesis_draft(
        list,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin())
    .finish();
    scratch.append(&genesis).unwrap();
    let mut entries = vec![genesis];
    for (node, role) in grants {
        let entry = scratch
            .draft(Body::Grant {
                node: *node,
                role: *role,
                key: person(*node as u8).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        entries.push(entry);
    }
    entries
}

/// alice (node 1) and bob (node 2) are writers, carol (node 3) a reader.
fn standard() -> Vec<Entry> {
    entries_of(CALENDAR, &[(1, 2), (2, 2), (3, 1)])
}

fn host_with(entries: &[Entry], own: Option<u64>) -> Trust {
    let trust = Trust::open(CALENDAR);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if let Some(node) = own {
        trust.set_identity(node, person(node as u8));
    }
    trust
}

fn host(own: Option<u64>) -> Trust {
    host_with(&standard(), own)
}

fn write_at(author: u64, id: u64, wall_time: i64) -> String {
    json!({"id":id.to_string(),"field":"3","value":"1","text":"",
           "wallTime":wall_time.to_string(),"counter":"0","nodeId":author.to_string()})
    .to_string()
}

fn sent(host: &mut Trust, event: &str, data: &str) -> String {
    match host.seal("#server", event, data) {
        Ok(Some(sent)) => sent,
        other => panic!("expected {event} to leave, got {other:?}"),
    }
}

/// The hub's floor as it leaves for the members.
fn floor_leaving(hub: &mut Trust) -> Value {
    serde_json::from_str(&sent(hub, "todo.floor", FLOOR)).unwrap()
}

/// A hub that holds the checkpoints of alice and bob for the instant 30000 and the digest 7.
fn settled_hub() -> Trust {
    let hub = host(None);
    for node in [1, 2] {
        hub.take_checkpoint(&host(Some(node)).checkpoint(30_000, 7).unwrap())
            .unwrap();
    }
    hub
}

#[test]
fn the_members_settle_an_instant_with_no_administrator() {
    let mut hub = settled_hub();
    let mut member = host(Some(1));
    let leaving = floor_leaving(&mut hub);
    assert!(leaving.get("cert").is_some(), "{leaving}");

    // A host that holds only the log, and has seen none of the checkpoints, checks it alone, and
    // the machine is told the floor as it made it.
    let admitted = member
        .admit("server", "todo.floor", &leaving.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&admitted).unwrap(),
        serde_json::from_str::<Value>(FLOOR).unwrap()
    );
}

#[test]
fn a_floor_without_the_word_of_every_writer_leaves_as_it_is_and_is_refused() {
    let mut hub = host(None);
    let mut member = host(Some(2));
    hub.take_checkpoint(&host(Some(1)).checkpoint(30_000, 7).unwrap())
        .unwrap();
    assert_eq!(hub.settlement(30_000, 7), None, "bob has not said");
    assert_eq!(hub.seal("#server", "todo.floor", FLOOR), Ok(None));
    let reason = member.admit("server", "todo.floor", FLOOR).unwrap_err();
    assert!(reason.contains("no settlement"), "{reason}");
}

#[test]
fn a_reader_makes_no_checkpoint() {
    let carol = host(Some(3));
    assert_eq!(
        carol.checkpoint(30_000, 7),
        Err(CheckpointError::NotAWriter { device: 3, log: 3 })
    );
    assert_eq!(
        host(None).checkpoint(30_000, 7),
        Err(CheckpointError::NoPerson)
    );
}

#[test]
fn a_settlement_is_for_one_instant_and_one_digest() {
    let hub = host(None);
    hub.take_checkpoint(&host(Some(1)).checkpoint(30_000, 7).unwrap())
        .unwrap();
    hub.take_checkpoint(&host(Some(2)).checkpoint(30_000, 8).unwrap())
        .unwrap();
    assert_eq!(hub.settlement(30_000, 7), None);
    assert_eq!(hub.settlement(30_000, 8), None);

    // A settlement of one instant is no word for another, nor for another digest.
    let good = settled_hub().settlement(30_000, 7).unwrap();
    let mut member = host(Some(1));
    for floor in [
        r#"{"before":"30001","hash":"7"}"#,
        r#"{"before":"30000","hash":"8"}"#,
    ] {
        let mut value: Value = serde_json::from_str(floor).unwrap();
        value["cert"] = good.to_value();
        let reason = member
            .admit("server", "todo.floor", &value.to_string())
            .unwrap_err();
        assert!(
            reason.contains("another instant or digest"),
            "{floor} {reason}"
        );
    }
}

#[test]
fn a_hub_cannot_make_a_settlement_out_of_what_is_not_the_writers() {
    let good = settled_hub().settlement(30_000, 7).unwrap();
    let alice = good.checkpoints.iter().find(|c| c.device == 1).unwrap();
    let mut member = host(Some(2));
    let admit = |member: &mut Trust, checkpoints: Vec<Checkpoint>| {
        let mut value: Value = serde_json::from_str(FLOOR).unwrap();
        value["cert"] = Settlement { checkpoints }.to_value();
        member.admit("server", "todo.floor", &value.to_string())
    };

    // Whole, it holds.
    assert!(admit(&mut member, good.checkpoints.clone()).is_ok());
    // One writer short.
    let reason = admit(&mut member, vec![alice.clone()]).unwrap_err();
    assert!(reason.contains("writers"), "{reason}");
    // One writer twice, in the place of the other.
    let reason = admit(&mut member, vec![alice.clone(), alice.clone()]).unwrap_err();
    assert!(reason.contains("twice"), "{reason}");
    // A reader's word, signed with the reader's own key, is no writer's.
    let mut carols = alice.clone();
    carols.device = 3;
    carols.signature = person(3).sign(&carols.signing_bytes());
    let reason = admit(
        &mut member,
        vec![alice.clone(), good.checkpoints[1].clone(), carols],
    )
    .unwrap_err();
    assert!(reason.contains("does not hold"), "{reason}");
    // None at all.
    assert!(admit(&mut member, Vec::new()).is_err());
    // Not checkpoints.
    let mut value: Value = serde_json::from_str(FLOOR).unwrap();
    value["cert"] = json!("trust me");
    let reason = member
        .admit("server", "todo.floor", &value.to_string())
        .unwrap_err();
    assert!(reason.contains("not checkpoints"), "{reason}");
}

#[test]
fn a_checkpoint_that_is_forged_or_foreign_is_not_taken_in() {
    let hub = host(None);
    let mut alice = host(Some(1));
    sent(&mut alice, "todo.write", &write_at(1, 1, 1000));
    let genuine = alice.checkpoint(30_000, 7).unwrap();
    assert_eq!(genuine.heads.len(), 1);

    // Another digest under the same signature.
    let mut moved = genuine.clone();
    moved.digest = 8;
    assert_eq!(
        hub.take_checkpoint(&moved),
        Err(CheckpointError::BadSignature)
    );
    // A head that was not signed.
    let mut moved = genuine.clone();
    moved.heads.clear();
    assert_eq!(
        hub.take_checkpoint(&moved),
        Err(CheckpointError::BadSignature)
    );
    // Another device's name on it.
    let mut moved = genuine.clone();
    moved.device = 2;
    assert_eq!(
        hub.take_checkpoint(&moved),
        Err(CheckpointError::BadSignature)
    );
    // A key the log did not give the device: a host that signs as node 1 with another key makes
    // no checkpoint, and a checkpoint signed so by hand is not taken in.
    let impostor = host_with(&standard(), None);
    impostor.set_identity(1, person(99));
    assert_eq!(
        impostor.checkpoint(30_000, 7),
        Err(CheckpointError::BadSignature)
    );
    let mut forged = genuine.clone();
    forged.signature = person(99).sign(&forged.signing_bytes());
    assert_eq!(
        hub.take_checkpoint(&forged),
        Err(CheckpointError::BadSignature)
    );
    // A node that is no member.
    let stranger = host_with(&standard(), None);
    stranger.set_identity(50, person(50));
    assert_eq!(
        stranger.checkpoint(30_000, 7),
        Err(CheckpointError::NotAWriter { device: 50, log: 3 })
    );
    // Another list.
    let other = Trust::open("work");
    for entry in entries_of("work", &[(1, 2)]) {
        other.append(&entry).unwrap();
    }
    other.set_identity(1, person(1));
    assert_eq!(
        hub.take_checkpoint(&other.checkpoint(30_000, 7).unwrap()),
        Err(CheckpointError::WrongList)
    );
    // An entry of the log this host has not reached.
    let ahead = host_with(
        &entries_of(CALENDAR, &[(1, 2), (2, 2), (3, 1), (4, 2)]),
        Some(1),
    );
    assert_eq!(
        hub.take_checkpoint(&ahead.checkpoint(30_000, 7).unwrap()),
        Err(CheckpointError::LogNotHeld { log: 4 })
    );
    // What is held is the genuine one, once.
    assert_eq!(hub.take_checkpoint(&genuine), Ok(true));
    assert_eq!(hub.take_checkpoint(&genuine), Ok(false));
}

/// An operation of alice's that a relay did not give bob: her checkpoint names it, and he asks
/// another relay for it. What he is given is checked as if it came from anyone.
#[test]
fn a_member_sees_the_operations_a_relay_did_not_give_and_fetches_them_elsewhere() {
    let mut alice = host(Some(1));
    let ops = [
        sent(&mut alice, "todo.write", &write_at(1, 1, 1000)),
        sent(&mut alice, "todo.write", &write_at(1, 2, 1001)),
        sent(&mut alice, "todo.write", &write_at(1, 3, 1002)),
    ];
    let mut honest_relay = host(None);
    let mut withholding_relay = host(None);
    for op in &ops {
        honest_relay.admit("alice", "todo.write", op).unwrap();
    }
    // The relay that withholds passes on the first and the last, and not the one between.
    withholding_relay
        .admit("alice", "todo.write", &ops[0])
        .unwrap();
    withholding_relay
        .admit("alice", "todo.write", &ops[2])
        .unwrap();

    let mut bob = host(Some(2));
    for op in [&ops[0], &ops[2]] {
        bob.admit("withholding", "todo.write", op).unwrap();
    }
    assert_eq!(bob.heads().iter().map(|h| h.seq).collect::<Vec<_>>(), [1]);
    assert_eq!(bob.divergences(), vec![], "nothing is named yet");

    bob.take_checkpoint(&alice.checkpoint(30_000, 7).unwrap())
        .unwrap();
    assert_eq!(
        bob.divergences(),
        vec![Divergence::Missing {
            node: 1,
            held: 1,
            named: 3,
            by: 1
        }]
    );

    // The relay that withheld has the numbers it was given and no more; the honest one has all.
    assert_eq!(withholding_relay.operations(1, 2, 2), Vec::new());
    let fetched = honest_relay.operations(1, 2, 3);
    assert_eq!(fetched.len(), 2);
    assert_eq!(honest_relay.operations(1, 3, 2), Vec::new());
    for (event, data) in &fetched {
        assert_eq!(event, "todo.write");
        bob.admit("honest", event, data).unwrap();
    }
    assert_eq!(bob.divergences(), vec![]);
    assert_eq!(bob.heads().iter().map(|h| h.seq).collect::<Vec<_>>(), [3]);

    // What a relay hands over is checked: a changed operation is not taken in.
    let mut altered: Value = serde_json::from_str(&fetched[0].1).unwrap();
    altered["value"] = json!("2");
    assert!(host(Some(2))
        .admit("honest", "todo.write", &altered.to_string())
        .is_err());
}

/// A checkpoint that names another operation than the one held at a number is the sign that the
/// device signed two histories, or that someone is lying about it; the host does not settle on it.
#[test]
fn a_checkpoint_that_names_another_operation_is_a_difference_and_settles_nothing() {
    // Two hosts hold alice's key, as a copy of her device that forked would.
    let mut first = host(Some(1));
    let mut second = host(Some(1));
    let one = sent(&mut first, "todo.write", &write_at(1, 1, 1000));
    sent(&mut second, "todo.write", &write_at(1, 9, 1000));

    let mut bob = host(Some(2));
    bob.admit("relay", "todo.write", &one).unwrap();
    bob.take_checkpoint(&second.checkpoint(30_000, 7).unwrap())
        .unwrap();
    assert_eq!(
        bob.divergences(),
        vec![Divergence::Differs {
            node: 1,
            seq: 1,
            by: 1
        }]
    );

    // A settlement that carries the other history is refused by the host that holds this one.
    let hub = host(None);
    hub.take_checkpoint(&second.checkpoint(30_000, 7).unwrap())
        .unwrap();
    hub.take_checkpoint(&host(Some(2)).checkpoint(30_000, 7).unwrap())
        .unwrap();
    let mut value: Value = serde_json::from_str(FLOOR).unwrap();
    value["cert"] = hub.settlement(30_000, 7).unwrap().to_value();
    let reason = bob
        .admit("server", "todo.floor", &value.to_string())
        .unwrap_err();
    assert!(reason.contains("another operation"), "{reason}");
}

/// A device taken off is not asked, and a settlement made before is still one after.
#[test]
fn a_device_taken_off_is_not_asked_and_an_earlier_settlement_stays_true() {
    let before_removal = settled_hub().settlement(30_000, 7).unwrap();

    let mut entries = standard();
    let scratch = host_with(&entries, None);
    entries.push(
        scratch
            .draft(Body::Grant {
                node: 2,
                role: 0,
                key: person(2).public(),
                final_seq: Some(0),
            })
            .sign(&admin())
            .finish(),
    );

    // Under the entry that took bob off, alice's word alone is the writers'.
    let hub = host_with(&entries, None);
    hub.take_checkpoint(&host_with(&entries, Some(1)).checkpoint(40_000, 11).unwrap())
        .unwrap();
    assert!(hub.settlement(40_000, 11).is_some());

    // The settlement under the earlier entry is a settlement of that entry's writers, still.
    let mut member = host_with(&entries, Some(1));
    let mut value: Value = serde_json::from_str(FLOOR).unwrap();
    value["cert"] = before_removal.to_value();
    assert!(member
        .admit("server", "todo.floor", &value.to_string())
        .is_ok());
}

/// A host that holds an operation of a device at 1 and 3 and not 2 holds the chain in full only to 1.
#[test]
fn the_head_of_a_chain_is_the_last_number_held_with_every_one_before_it() {
    let mut alice = host(Some(1));
    let ops: Vec<String> = (1..=3)
        .map(|n| sent(&mut alice, "todo.write", &write_at(1, n, 1000 + n as i64)))
        .collect();
    let mut hub = host(None);
    assert_eq!(hub.heads(), Vec::new());
    hub.admit("a", "todo.write", &ops[2]).unwrap();
    assert_eq!(hub.heads(), Vec::new(), "3 without 1 is no head");
    hub.admit("a", "todo.write", &ops[0]).unwrap();
    assert_eq!(hub.heads().len(), 1);
    assert_eq!(hub.heads()[0].seq, 1);
    hub.admit("a", "todo.write", &ops[1]).unwrap();
    assert_eq!(hub.heads()[0].seq, 3);
    assert_eq!(alice.heads(), hub.heads(), "the same chain, the same head");
}

/// A host that restarts keeps the checkpoints it held and the operations it can give, so that what
/// it knew to be withheld is still known, and a settlement is still one.
#[test]
fn what_a_host_holds_of_its_members_checkpoints_survives_a_restart() {
    let hub = settled_hub();
    let text = hub.to_text();
    let restarted = host(None);
    assert_eq!(restarted.restore_from_text(&text), 2);
    assert!(restarted.settlement(30_000, 7).is_some());

    // Against a log in which the devices were never writers, none of it holds.
    let changed = host_with(&entries_of(CALENDAR, &[(1, 1), (2, 1)]), None);
    assert_eq!(changed.restore_from_text(&text), 0);
}
