// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a host that does not trust the host beside it lets through. What a device sends is an
// operation of its chain: the event, the number it has among the device's operations, the one it
// follows, the entry of the membership log it was made under, and the device's signature of all of
// it. A host admits an operation when the entry it names is the one the host holds, its author was a
// writer under that entry, the administrators accepted the device's operations up to its number,
// and the signature is true; whatever order the operations came in, and whatever time the device
// claims, which decides nothing. What anyone else does with it (a changed field, a key that is not
// the author's, a reader, a stranger, another list, a device taken off, a device that signed two
// things at one number, a host shown two histories of the log) is refused, and what a hub relays it
// relays as the operation the member made.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::membership::{Body, Entry, Log, LogError, Profile};
use sce_rust_trust::{Identity, Trust, TrustError};
use serde_json::{json, Value};

const CALENDAR: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn genesis_of(list: &str, profile: Profile) -> Entry {
    Log::genesis_draft(
        list,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile,
        },
    )
    .sign(&admin())
    .finish()
}

/// The entries of a log of `list` that the administrator signed: the genesis, and a grant for
/// each of `grants`, as `(node, role, key seed)`.
fn entries_of(list: &str, profile: Profile, grants: &[(u64, u8, u8)]) -> Vec<Entry> {
    let scratch = Trust::open(list);
    let genesis = genesis_of(list, profile);
    scratch.append(&genesis).unwrap();
    let mut entries = vec![genesis];
    for (node, role, seed) in grants {
        let entry = scratch
            .draft(Body::Grant {
                node: *node,
                role: *role,
                key: person(*seed).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        entries.push(entry);
    }
    entries
}

/// alice (node 1, writer), bob (node 2, writer) and carol (node 3, reader).
fn standard() -> Vec<Entry> {
    entries_of(
        CALENDAR,
        Profile::Untrusted,
        &[(1, 2, 1), (2, 2, 2), (3, 1, 3)],
    )
}

fn host_with(entries: &[Entry], own: Option<(u64, Identity)>) -> Trust {
    let trust = Trust::open(CALENDAR);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if let Some((node, identity)) = own {
        trust.set_identity(node, identity);
    }
    trust
}

/// A host of the standard list, for the person `own` when there is one.
fn host(own: Option<(u64, Identity)>) -> Trust {
    host_with(&standard(), own)
}

/// The entry an administrator signs to take the device of `node` (which signs with `seed`) off,
/// its chain ending at `final_seq`, drafted on the log `on` holds.
fn removal(on: &Trust, node: u64, seed: u8, final_seq: u64) -> Entry {
    on.draft(Body::Grant {
        node,
        role: 0,
        key: person(seed).public(),
        final_seq: Some(final_seq),
    })
    .sign(&admin())
    .finish()
}

fn write(author: u64, value: i64) -> String {
    write_at(author, value, 1000)
}

fn write_at(author: u64, value: i64, wall_time: i64) -> String {
    json!({"id":"1","field":"3","value":value.to_string(),"text":"",
           "wallTime":wall_time.to_string(),"counter":"0","nodeId":author.to_string()})
    .to_string()
}

/// A tombstone of the write of node 2 at 1000, as the machine of `by` sends it.
fn supersede(by: u64) -> String {
    json!({"id":1,"field":3,"wallTime":"1000","counter":0,"nodeId":"2","by":by.to_string()})
        .to_string()
}

fn send(host: &mut Trust, event: &str, data: &str) -> Result<Option<String>, String> {
    host.seal("#server", event, data)
}

/// What a host sent: the data, and a test that it sent any fails with the reason.
fn sent(host: &mut Trust, event: &str, data: &str) -> String {
    match send(host, event, data) {
        Ok(Some(sent)) => sent,
        other => panic!("expected {event} to leave, got {other:?}"),
    }
}

#[test]
fn a_write_a_member_signed_is_admitted_and_arrives_as_the_machine_wrote_it() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let on_the_wire: Value = serde_json::from_str(&sealed).unwrap();
    for field in ["sig", "seq", "prev", "mh", "mhh"] {
        assert!(on_the_wire.get(field).is_some(), "{field}: {sealed}");
    }
    assert_eq!(on_the_wire["seq"], "1");
    let admitted = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    let wrote: Value = serde_json::from_str(&write(1, 10)).unwrap();
    assert_eq!(
        admitted, wrote,
        "the machine is told what it was sent, with nothing the host wrote"
    );
}

#[test]
fn what_is_not_a_signed_event_goes_through_untouched() {
    let mut hub = host(None);
    assert_eq!(hub.admit("alice", "todo.node", "{}"), Ok(None));
    assert_eq!(send(&mut hub, "todo.node", "{}"), Ok(None));
}

#[test]
fn a_write_with_no_signature_or_a_changed_field_or_another_key_is_refused() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let unsigned = write(1, 10);
    assert!(hub.admit("alice", "todo.write", &unsigned).is_err());

    let sealed = sent(&mut alice, "todo.write", &unsigned);
    let edited = sealed.replace("\"10\"", "\"11\"");
    assert_ne!(edited, sealed);
    let reason = hub.admit("alice", "todo.write", &edited).unwrap_err();
    assert!(reason.contains("not node 1's"), "{reason}");

    // A link that was edited is as good as a field: the number, the entry it names.
    for (field, other) in [("seq", "2"), ("mh", "1")] {
        let mut value: Value = serde_json::from_str(&sealed).unwrap();
        value[field] = json!(other);
        assert!(
            hub.admit("alice", "todo.write", &value.to_string())
                .is_err(),
            "{field}"
        );
    }

    // Bob's host does not sign in alice's name, and a host that holds another key for node 1
    // does, which the key the log gives node 1 does not verify.
    let mut bob = host(Some((2, person(2))));
    assert!(send(&mut bob, "todo.write", &write(1, 10)).is_err());
    let mut forger = host(Some((1, person(5))));
    let forged = sent(&mut forger, "todo.write", &write(1, 10));
    assert!(hub.admit("x", "todo.write", &forged).is_err());
}

#[test]
fn a_reader_and_a_stranger_may_not_write_whatever_they_sign() {
    let mut hub = host(None);
    for (node, seed) in [(3u64, 3u8), (4, 4)] {
        let mut copy = host(Some((node, person(seed))));
        let sealed = sent(&mut copy, "todo.write", &write(node, 1));
        let reason = hub.admit("x", "todo.write", &sealed).unwrap_err();
        assert!(
            reason.contains("may not write") || reason.contains("not a member"),
            "{reason}"
        );
    }
}

#[test]
fn a_write_is_good_in_its_own_list_only() {
    let mut alice = host(Some((1, person(1))));
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let work = entries_of("work", Profile::Untrusted, &[(1, 2, 1)]);
    let mut other = Trust::open("work");
    for entry in &work {
        other.append(entry).unwrap();
    }
    assert!(other.admit("x", "todo.write", &sealed).is_err());
    assert_eq!(
        host(None).append(&work[0]),
        Err(LogError::WrongList),
        "an entry of another list is not taken in"
    );
}

#[test]
fn a_hub_hands_on_what_a_member_wrote_as_the_operation_the_member_made() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let mut bob = host(Some((2, person(2))));
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    // The hub's machine sends it on to bob as the write it holds.
    let relayed = sent(&mut hub, "todo.write", &for_the_machine);
    assert_eq!(
        serde_json::from_str::<Value>(&relayed).unwrap(),
        serde_json::from_str::<Value>(&sealed).unwrap(),
        "the very operation, with its number and its signature"
    );
    assert!(bob.admit("server", "todo.write", &relayed).is_ok());
    // A write it never saw it cannot send on: it holds no operation of node 1's for it and signs
    // nothing of its own, and it says so instead of sending something else.
    let reason = send(&mut hub, "todo.write", &write(1, 99)).unwrap_err();
    assert!(reason.contains("signs none in their name"), "{reason}");
}

/// The operations of one device come in numbers and name the one before them; a host admits each
/// that holds in whatever order they arrive, and one that arrives twice is admitted once.
#[test]
fn operations_are_admitted_in_whatever_order_they_come_and_once() {
    let mut alice = host(Some((1, person(1))));
    let wire: Vec<String> = (0..4)
        .map(|n| sent(&mut alice, "todo.write", &write_at(1, n, 1000 + n)))
        .collect();
    let seqs: Vec<Value> = wire
        .iter()
        .map(|w| serde_json::from_str::<Value>(w).unwrap()["seq"].clone())
        .collect();
    assert_eq!(seqs, vec!["1", "2", "3", "4"]);

    let mut hub = host(None);
    for n in [2, 0, 3, 1, 2] {
        assert!(hub.admit("x", "todo.write", &wire[n]).is_ok(), "{n}");
    }
    assert_eq!(hub.last_seq(1), 4);
    assert!(hub.equivocating().is_empty());
}

/// A device that signs two different operations at one number has forked its own chain, and a
/// host that sees both says so and takes nothing of the device from there, though it goes on with
/// everyone else.
#[test]
fn a_device_that_signed_two_things_at_one_number_is_frozen() {
    let mut alice = host(Some((1, person(1))));
    let first = sent(&mut alice, "todo.write", &write_at(1, 1, 1000));
    // The same device on another host (a restored backup) writes something else at number 1.
    let mut twin = host(Some((1, person(1))));
    let other = sent(&mut twin, "todo.write", &write_at(1, 2, 1001));

    let mut hub = host(None);
    assert!(hub.admit("x", "todo.write", &first).is_ok());
    let reason = hub.admit("x", "todo.write", &other).unwrap_err();
    assert!(reason.contains("two different operations at 1"), "{reason}");
    assert_eq!(hub.equivocating(), vec![(1, 1)]);

    // Nothing of node 1's from number 1 on, not even what was held, and nothing else is touched.
    let reason = hub.admit("x", "todo.write", &first).unwrap_err();
    assert!(reason.contains("signed two operations at 1"), "{reason}");
    let mut bob = host(Some((2, person(2))));
    assert!(hub
        .admit(
            "x",
            "todo.write",
            &sent(&mut bob, "todo.write", &write(2, 5))
        )
        .is_ok());
}

/// A device's stamps do not go back along its chain: what it wrote later is not given an earlier
/// time than what it had already written, whatever number order they came in.
#[test]
fn a_stamp_that_goes_back_along_a_chain_is_refused() {
    let mut alice = host(Some((1, person(1))));
    let early = sent(&mut alice, "todo.write", &write_at(1, 1, 2000));
    let backdated = sent(&mut alice, "todo.write", &write_at(1, 2, 1500));
    let later = sent(&mut alice, "todo.write", &write_at(1, 3, 3000));

    let mut hub = host(None);
    assert!(hub.admit("x", "todo.write", &later).is_ok());
    assert!(hub.admit("x", "todo.write", &early).is_ok());
    let reason = hub.admit("x", "todo.write", &backdated).unwrap_err();
    assert!(reason.contains("stamp goes back"), "{reason}");
}

/// A device is taken off at the last operation of its chain that the administrators accept. What
/// it made up to there stays and reaches any host in any order; what it made past it is refused
/// by every host, though it names an entry of the log from before it was taken off.
#[test]
fn a_device_taken_off_is_cut_at_the_last_operation_accepted() {
    let entries = standard();
    let mut alice = host_with(&entries, Some((1, person(1))));
    let ops: Vec<String> = (0..4)
        .map(|n| sent(&mut alice, "todo.write", &write_at(1, n, 1000 + n)))
        .collect();

    // The hub has seen the first two, and the administrator ends the chain there.
    let mut hub = host_with(&entries, None);
    for op in &ops[..2] {
        assert!(hub.admit("alice", "todo.write", op).is_ok());
    }
    assert_eq!(hub.last_seq(1), 2);
    let off = removal(&hub, 1, 1, hub.last_seq(1));
    assert_eq!(hub.append(&off), Ok(true));
    assert_eq!(hub.role(1), 0);

    // The device did not hear of it: it names the entry it knew, and goes on, past the end.
    for op in &ops[2..] {
        let reason = hub.admit("alice", "todo.write", op).unwrap_err();
        assert!(reason.contains("past 2"), "{reason}");
    }
    // What was admitted is admitted again, and a host that joins after the removal takes the same
    // two in the other order and none of the rest.
    assert!(hub.admit("alice", "todo.write", &ops[1]).is_ok());
    let mut late = host_with(&entries, None);
    late.append(&off).unwrap();
    assert!(late.admit("x", "todo.write", &ops[1]).is_ok());
    assert!(late.admit("x", "todo.write", &ops[0]).is_ok());
    assert!(late.admit("x", "todo.write", &ops[2]).is_err());
}

/// A grant that takes writing away has to say where the chain ends, and the entry that does is
/// the whole of a removal: the device cannot go back to an entry before it to write on.
#[test]
fn a_removed_device_cannot_write_on_by_naming_an_older_entry() {
    let entries = standard();
    let mut alice = host_with(&entries, Some((1, person(1))));
    let mut hub = host_with(&entries, None);
    let first = sent(&mut alice, "todo.write", &write_at(1, 1, 1000));
    assert!(hub.admit("alice", "todo.write", &first).is_ok());
    hub.append(&removal(&hub, 1, 1, hub.last_seq(1))).unwrap();

    // Alice's own host learns of the removal too and writes: it names the newest entry, where
    // her role is 0, and so does a host that backdates and names the old one.
    let mut told = host_with(&entries, Some((1, person(1))));
    told.append(&removal(&told, 1, 1, 1)).unwrap();
    told.restore_signatures_from_text(&alice.signatures_to_text());
    let next = sent(&mut told, "todo.write", &write_at(1, 2, 1001));
    let reason = hub.admit("alice", "todo.write", &next).unwrap_err();
    assert!(
        reason.contains("may not write") || reason.contains("past 1"),
        "{reason}"
    );
    let backdated = sent(&mut alice, "todo.write", &write_at(1, 3, 1002));
    let reason = hub.admit("alice", "todo.write", &backdated).unwrap_err();
    assert!(reason.contains("past 1"), "{reason}");
}

/// A grant that does not say where the chain ends is refused where it is made.
#[test]
fn a_grant_that_takes_writing_away_must_say_where_the_chain_ends() {
    let hub = host(None);
    let without = hub
        .draft(Body::Grant {
            node: 1,
            role: 0,
            key: person(1).public(),
            final_seq: None,
        })
        .sign(&admin())
        .finish();
    assert_eq!(
        hub.append(&without),
        Err(LogError::Invalid(
            "a grant that takes writing away carries the last operation accepted"
        ))
    );
}

/// A new key for a node (a lost device replaced) begins a new chain after the end the grant set:
/// the old key's operations up to it stay, its later ones are refused, and the new key's are
/// numbered past it and name nothing before them.
#[test]
fn a_replaced_key_begins_a_new_chain_where_the_old_one_ended() {
    let base = standard();
    let mut old = host_with(&base, Some((1, person(1))));
    let old_ops: Vec<String> = (0..3)
        .map(|n| sent(&mut old, "todo.write", &write_at(1, n, 1000 + n)))
        .collect();

    // The administrator replaces node 1's key and ends its chain at 2.
    let scratch = host_with(&base, None);
    let swap = scratch
        .draft(Body::Grant {
            node: 1,
            role: 2,
            key: person(11).public(),
            final_seq: Some(2),
        })
        .sign(&admin())
        .finish();
    let mut all = base.clone();
    all.push(swap);

    let mut hub = host_with(&all, None);
    assert!(hub.admit("x", "todo.write", &old_ops[0]).is_ok());
    assert!(hub.admit("x", "todo.write", &old_ops[1]).is_ok());
    let reason = hub.admit("x", "todo.write", &old_ops[2]).unwrap_err();
    assert!(reason.contains("past 2"), "{reason}");

    let mut new = host_with(&all, Some((1, person(11))));
    let first = sent(&mut new, "todo.write", &write_at(1, 9, 500));
    let second = sent(&mut new, "todo.write", &write_at(1, 10, 501));
    let numbers: Vec<Value> = [&first, &second]
        .iter()
        .map(|w| serde_json::from_str::<Value>(w).unwrap()["seq"].clone())
        .collect();
    assert_eq!(numbers, vec!["3", "4"], "past the end the grant set");
    assert!(hub.admit("x", "todo.write", &first).is_ok());
    assert!(hub.admit("x", "todo.write", &second).is_ok());
    // The new chain's stamps are not held to the old one's: it is another device's clock.
    assert_eq!(hub.last_seq(1), 4);

    // The old key cannot claim the numbers of the new chain, and the new key cannot claim the old's.
    let mut old_late = host_with(&base, Some((1, person(1))));
    for _ in 0..3 {
        sent(&mut old_late, "todo.write", &write_at(1, 0, 1));
    }
    let past = sent(&mut old_late, "todo.write", &write_at(1, 1, 2000));
    assert!(hub.admit("x", "todo.write", &past).is_err());
}

#[test]
fn entries_that_arrive_out_of_order_wait_for_the_ones_before_them() {
    let entries = standard();
    let trust = Trust::open(CALENDAR);
    trust.append(&entries[0]).unwrap();
    assert_eq!(
        trust.append(&entries[2]),
        Err(LogError::Gap { expected: 1 }),
        "the grant of node 2 needs the grant of node 1 first"
    );
    assert_eq!(trust.append(&entries[1]), Ok(true));
    assert_eq!(trust.append(&entries[2]), Ok(true));
    assert_eq!(trust.role(1), 2);
    assert_eq!(trust.role(2), 2);
}

/// An operation names an entry of the log; a host behind on the log refuses what names an entry it
/// does not hold, and takes it once it does.
#[test]
fn an_operation_that_names_an_entry_the_host_does_not_hold_waits_for_it() {
    let entries = standard();
    let mut alice = host_with(&entries, Some((1, person(1))));
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let mut behind = host_with(&entries[..3], None);
    let reason = behind.admit("x", "todo.write", &sealed).unwrap_err();
    assert!(reason.contains("does not hold as it is"), "{reason}");
    behind.append(&entries[3]).unwrap();
    assert!(behind.admit("x", "todo.write", &sealed).is_ok());
}

#[test]
fn a_person_invited_by_the_hash_takes_that_genesis_and_no_other() {
    let real = genesis_of(CALENDAR, Profile::Untrusted);
    let trust = Trust::invited(CALENDAR, real.hash());
    let impostor = Log::genesis_draft(
        CALENDAR,
        Body::Genesis {
            administrators: vec![person(66).public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&person(66))
    .finish();
    assert_eq!(trust.append(&impostor), Err(LogError::NotTheGenesis));
    assert!(trust.administrators().is_empty());
    assert_eq!(trust.append(&real), Ok(true));
    assert_eq!(trust.administrators(), vec![admin().public()]);
}

/// A host that is shown two entries at one position has proof that the chain forked: it says
/// where, and takes in no write, note or settled instant until an administrator resolves it. A
/// host that is behind and is shown the later entry is only told what it is missing.
#[test]
fn a_fork_of_the_log_freezes_the_host_that_sees_it() {
    let entries = standard();
    let mut hub = host_with(&entries, None);
    let mut alice = host(Some((1, person(1))));
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    assert!(hub.admit("alice", "todo.write", &sealed).is_ok());
    assert_eq!(hub.forked(), None);

    // A second history of position 3: another grant the administrator also signed.
    let other = host_with(&entries[..3], None)
        .draft(Body::Grant {
            node: 3,
            role: 2,
            key: person(3).public(),
            final_seq: None,
        })
        .sign(&admin())
        .finish();
    assert_eq!(other.index, 3);
    assert_eq!(hub.append(&other), Err(LogError::Fork { index: 3 }));
    assert_eq!(hub.forked(), Some(3));
    let reason = hub.admit("alice", "todo.write", &sealed).unwrap_err();
    assert!(reason.contains("forked at 3"), "{reason}");
    let reason = send(&mut hub, "todo.write", &write(1, 10)).unwrap_err();
    assert!(reason.contains("forked at 3"), "{reason}");
    // What is not a write goes through as it did.
    assert_eq!(hub.admit("alice", "todo.node", "{}"), Ok(None));
}

#[test]
fn the_roster_the_log_makes_is_what_the_machine_is_told() {
    let mut entries = standard();
    let scratch = host_with(&entries, None);
    entries.push(removal(&scratch, 3, 3, 0));
    let hub = host_with(&entries, None);
    let roles: Vec<(String, String)> = [(1, 2), (2, 2), (3, 0)]
        .iter()
        .map(|(node, role)| {
            (
                "todo.role".to_string(),
                format!("{{\"node\":\"{node}\",\"role\":{role}}}"),
            )
        })
        .collect();
    assert_eq!(hub.role_events(), roles);
}

#[test]
fn the_operations_a_host_relays_survive_a_restart_when_they_are_still_true() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    let kept = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&kept), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&sent(&mut restarted, "todo.write", &for_the_machine))
            .unwrap(),
        serde_json::from_str::<Value>(&sealed).unwrap(),
        "the same operation again, with the same number"
    );

    let signature = serde_json::from_str::<Value>(&sealed).unwrap()["sig"]
        .as_str()
        .unwrap()
        .to_string();
    let mut flipped = signature.clone();
    flipped.replace_range(0..1, if signature.starts_with('A') { "B" } else { "A" });
    let tampered = kept.replace(&signature, &flipped);
    assert_ne!(tampered, kept);
    let mut again = host(None);
    assert_eq!(again.restore_signatures_from_text(&tampered), 0);
    assert!(send(&mut again, "todo.write", &for_the_machine).is_err());
}

/// A host that comes back from its store goes on with its person's chain where it was, and does
/// not reuse a number.
#[test]
fn a_person_host_that_restarts_goes_on_with_the_next_number() {
    let mut alice = host(Some((1, person(1))));
    let first = sent(&mut alice, "todo.write", &write_at(1, 1, 1000));
    let second = sent(&mut alice, "todo.write", &write_at(1, 2, 1001));
    let kept = alice.signatures_to_text();

    let mut restarted = host(Some((1, person(1))));
    assert_eq!(restarted.restore_signatures_from_text(&kept), 2);
    let third = sent(&mut restarted, "todo.write", &write_at(1, 3, 1002));
    let value: Value = serde_json::from_str(&third).unwrap();
    assert_eq!(value["seq"], "3");
    let mut hub = host(None);
    for op in [&first, &second, &third] {
        assert!(hub.admit("x", "todo.write", op).is_ok());
    }
}

#[test]
fn the_signatures_a_host_keeps_in_its_store_read_back_as_text() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    hub.attest_floor(&admin(), 30_000, 7).unwrap();
    let text = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&text), 2);
    assert!(matches!(
        send(&mut restarted, "todo.write", &for_the_machine),
        Ok(Some(_))
    ));
    assert!(matches!(
        send(
            &mut restarted,
            "todo.floor",
            r#"{"before":"30000","hash":"7"}"#
        ),
        Ok(Some(_))
    ));

    let other = host(None);
    assert_eq!(other.restore_signatures_from_text("not json"), 0);
    assert_eq!(other.restore_signatures_from_text(r#"[["x","y"]]"#), 0);
}

#[test]
fn a_floor_is_admitted_when_an_administrator_signed_that_instant_and_hash() {
    let mut hub = host(None);
    let mut person_host = host(Some((1, person(1))));
    let floor = r#"{"before":"30000","hash":"7"}"#;

    // With nothing attested the hub's floor leaves as it is, and a host that trusts nobody
    // refuses it.
    assert_eq!(send(&mut hub, "todo.floor", floor), Ok(None));
    assert!(person_host.admit("server", "todo.floor", floor).is_err());

    hub.attest_floor(&admin(), 30_000, 7).unwrap();
    let sealed = sent(&mut hub, "todo.floor", floor);
    let admitted = person_host
        .admit("server", "todo.floor", &sealed)
        .unwrap()
        .unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    assert_eq!(admitted, serde_json::from_str::<Value>(floor).unwrap());

    // The signature belongs to that instant and that hash.
    for other in [
        r#"{"before":"30001","hash":"7"}"#,
        r#"{"before":"30000","hash":"8"}"#,
    ] {
        let mut value: Value = serde_json::from_str(other).unwrap();
        value["sig"] = serde_json::from_str::<Value>(&sealed).unwrap()["sig"].clone();
        let reason = person_host
            .admit("server", "todo.floor", &value.to_string())
            .unwrap_err();
        assert!(
            reason.contains("no administrator signed"),
            "{other} {reason}"
        );
    }
}

#[test]
fn only_a_key_the_log_names_attests_a_floor() {
    let hub = host(None);
    let stranger = person(66);
    assert_eq!(
        hub.attest_floor(&stranger, 30_000, 7),
        Err(TrustError::UnknownIssuer)
    );
    assert_eq!(
        hub.hold_floor_signature(30_000, 7, stranger.sign(b"anything")),
        Err(TrustError::BadSignature)
    );
}

#[test]
fn a_floor_is_taken_unsigned_once_the_host_was_told_to() {
    let mut person_host = host(None);
    let floor = r#"{"before":"30000","hash":"7"}"#;
    assert!(person_host.admit("server", "todo.floor", floor).is_err());
    person_host.allow_unsigned(&["todo.floor"]);
    assert_eq!(person_host.admit("server", "todo.floor", floor), Ok(None));
}

/// The log says whether the hub is trusted, and a host follows it: under a trusted hub the
/// settled instant is taken as it comes, and a change to the profile is an entry like any other.
#[test]
fn the_log_says_whether_the_hub_is_trusted() {
    let trusted = entries_of(CALENDAR, Profile::TrustedHub, &[(1, 2, 1)]);
    let mut person_host = host_with(&trusted, None);
    assert_eq!(person_host.profile(), Profile::TrustedHub);
    let floor = r#"{"before":"30000","hash":"7"}"#;
    assert_eq!(person_host.admit("server", "todo.floor", floor), Ok(None));

    let mut untrusting = host(None);
    assert_eq!(untrusting.profile(), Profile::Untrusted);
    assert!(untrusting.admit("server", "todo.floor", floor).is_err());
    let to_trusted = untrusting
        .draft(Body::Profile(Profile::TrustedHub))
        .sign(&admin())
        .finish();
    assert_eq!(untrusting.append(&to_trusted), Ok(true));
    assert_eq!(untrusting.admit("server", "todo.floor", floor), Ok(None));
}

/// What a person's copy sends when it wrote over a value is its next operation; the hub takes it
/// in without the host's fields, and when its machine sends the tombstone again (as itself, on a
/// resync) the host puts back the operation of whoever really wrote over it, so that a host that
/// trusts nobody takes it from the hub.
#[test]
fn a_note_that_a_write_was_written_over_keeps_who_wrote_over_it() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let mut bob = host(Some((2, person(2))));

    let sealed = sent(&mut alice, "todo.supersede", &supersede(1));
    let admitted = hub
        .admit("alice", "todo.supersede", &sealed)
        .unwrap()
        .unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    for field in ["sig", "seq", "prev", "mh", "mhh", "at"] {
        assert!(admitted.get(field).is_none(), "{field}");
    }
    assert_eq!(admitted["by"], "1");

    // The hub's machine sends the tombstone again as itself, on a resync.
    let relayed = sent(&mut hub, "todo.supersede", &supersede(100));
    let relayed_value: Value = serde_json::from_str(&relayed).unwrap();
    assert_eq!(relayed_value["by"], "1", "who really wrote over it");
    assert_eq!(relayed_value["seq"], "1");
    assert!(bob.admit("server", "todo.supersede", &relayed).is_ok());

    // A note nobody made is not sent on, and one the host's person did not make is not signed.
    let mut other = host(None);
    assert!(send(&mut other, "todo.supersede", &supersede(100)).is_err());
    let mut carol = host(Some((3, person(3))));
    assert!(send(&mut carol, "todo.supersede", &supersede(1)).is_err());
}

/// A device's notes are in the chain of its writes, numbered with them, and are cut with them.
#[test]
fn a_note_is_numbered_in_the_chain_of_the_writes_and_cut_with_them() {
    let entries = standard();
    let mut alice = host_with(&entries, Some((1, person(1))));
    let write_op = sent(&mut alice, "todo.write", &write_at(1, 1, 1000));
    let note_op = sent(&mut alice, "todo.supersede", &supersede(1));
    assert_eq!(serde_json::from_str::<Value>(&note_op).unwrap()["seq"], "2");

    let mut hub = host_with(&entries, None);
    hub.admit("alice", "todo.write", &write_op).unwrap();
    hub.append(&removal(&hub, 1, 1, 1)).unwrap();
    let reason = hub.admit("alice", "todo.supersede", &note_op).unwrap_err();
    assert!(reason.contains("past 1"), "{reason}");
}

/// What a host keeps of the operations comes back from its store when it is still true: against a
/// log in which the device is no longer a writer at that entry, none of it does.
#[test]
fn the_notes_a_host_keeps_come_back_from_its_store() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = sent(&mut alice, "todo.supersede", &supersede(1));
    hub.admit("alice", "todo.supersede", &sealed).unwrap();
    let text = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&text), 1);
    assert!(matches!(
        send(&mut restarted, "todo.supersede", &supersede(100)),
        Ok(Some(_))
    ));

    // A log in which node 1 was never a writer keeps none of it.
    let changed = host_with(
        &entries_of(CALENDAR, Profile::Untrusted, &[(1, 1, 1)]),
        None,
    );
    assert_eq!(changed.restore_signatures_from_text(&text), 0);
}

/// The roster a hub's machine is told keeps a member who could write at some entry a writer,
/// where a person's machine is told the role they have now.
#[test]
fn the_hub_machine_is_told_who_ever_wrote() {
    let mut entries = standard();
    let scratch = host_with(&entries, None);
    entries.push(removal(&scratch, 1, 1, 0));
    let hub = host_with(&entries, None);
    let role_of = |events: Vec<(String, String)>, node: u64| {
        events
            .iter()
            .find(|(_, data)| data.contains(&format!("\"node\":\"{node}\"")))
            .map(|(_, data)| data.clone())
            .unwrap()
    };
    assert!(role_of(hub.role_events(), 1).contains("\"role\":0"));
    assert!(role_of(hub.hub_role_events(), 1).contains("\"role\":2"));
    assert!(role_of(hub.hub_role_events(), 3).contains("\"role\":1"));
}
