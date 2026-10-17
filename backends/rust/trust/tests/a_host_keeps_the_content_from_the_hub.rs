// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A list whose members hold a key the hub does not. The membership log begins an epoch, and from
// then on a write leaves its author's host with its number and text sealed under the key of that
// epoch, signed as it travels; the hub's host, which holds no key, takes it as it is and relays
// it as it is, and the host of another member opens it and tells its machine the write as its
// author made it. What a host will not take is a write that is not sealed, one sealed under an
// epoch the log has not begun, or one sealed under an epoch it holds no key of; and a host that
// holds no key of the epoch it is in writes nothing.
// A key is given to a member sealed so that only that member opens it, and it comes from a key
// the list's members or administrators hold. An epoch begins with an entry of the log, and a
// removed member is given none of the keys that follow.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{GrantError, Identity, RotateError, Trust};
use serde_json::{json, Value};

const CALENDAR: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

/// The log of a list in which alice (node 1) and bob (node 2) write and carol (node 3) reads, and
/// that has begun `epochs` epochs.
fn list(epochs: u32) -> Vec<Entry> {
    let scratch = Trust::open(CALENDAR);
    let genesis = Log::genesis_draft(
        CALENDAR,
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
    for (node, role) in [(1, 2), (2, 2), (3, 1)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
                key: person(node as u8).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        entries.push(entry);
    }
    for epoch in 1..=epochs {
        let entry = scratch.draft(Body::Epoch { epoch }).sign(&admin()).finish();
        scratch.append(&entry).unwrap();
        entries.push(entry);
    }
    entries
}

/// A host that holds the log `entries`, for the person `own` when there is one, with the keys
/// `keys` as `(epoch, fill byte)`.
fn host_in(entries: &[Entry], own: Option<(u64, u8)>, keys: &[(u32, u8)]) -> Trust {
    let trust = Trust::open(CALENDAR);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if let Some((node, seed)) = own {
        trust.set_identity(node, person(seed));
    }
    for (epoch, fill) in keys {
        trust.set_epoch_key(*epoch, [*fill; 32]);
    }
    trust
}

fn titled(author: u64, wall_time: i64, text: &str) -> String {
    json!({"id":1,"field":1,"value":"0","text":text,
           "wallTime":wall_time.to_string(),"counter":0,"nodeId":author.to_string()})
    .to_string()
}

fn numbered(author: u64, wall_time: i64, value: i64) -> String {
    json!({"id":2,"field":3,"value":value.to_string(),"text":"",
           "wallTime":wall_time.to_string(),"counter":0,"nodeId":author.to_string()})
    .to_string()
}

/// What a host sends for a write, or the reason it does not.
fn try_send(host: &mut Trust, data: &str) -> Result<Option<String>, String> {
    host.seal("#server", "todo.write", data)
}

fn send(host: &mut Trust, data: &str) -> String {
    match try_send(host, data) {
        Ok(Some(sent)) => sent,
        other => panic!("expected the write to leave, got {other:?}"),
    }
}

fn text_of(data: &str) -> String {
    serde_json::from_str::<Value>(data).unwrap()["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_write_leaves_sealed_and_a_member_reads_it_where_the_hub_cannot() {
    let one = list(1);
    let mut alice = host_in(&one, Some((1, 1)), &[(1, 5)]);
    let mut hub = host_in(&one, None, &[]);
    let mut bob = host_in(&one, Some((2, 2)), &[(1, 5)]);

    let wire = send(&mut alice, &titled(1, 1000, "buy milk"));
    assert!(text_of(&wire).starts_with('~'), "{wire}");
    assert!(!wire.contains("buy milk"));
    let on_wire: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(on_wire["value"], "0");

    // The hub's host holds no key: it admits what it can verify and its machine holds the
    // sealed text, which it sends on as it is.
    let held_by_the_hub = hub.admit("alice", "todo.write", &wire).unwrap().unwrap();
    assert_eq!(text_of(&held_by_the_hub), text_of(&wire));
    let relayed = send(&mut hub, &held_by_the_hub);
    assert_eq!(
        serde_json::from_str::<Value>(&relayed).unwrap(),
        on_wire,
        "the hub sends it on as it was made"
    );

    // Bob's host opens it, and his machine is told what alice wrote.
    let told = bob
        .admit("server", "todo.write", &relayed)
        .unwrap()
        .unwrap();
    let told: Value = serde_json::from_str(&told).unwrap();
    assert_eq!(told["text"], "buy milk");
    assert!(told.get("sig").is_none());

    // A number is sealed as the text is.
    let wire = send(&mut alice, &numbered(1, 1001, -7));
    assert_eq!(text_of(&wire).chars().next(), Some('~'));
    assert_eq!(serde_json::from_str::<Value>(&wire).unwrap()["value"], "0");
    let told = bob.admit("server", "todo.write", &wire).unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&told).unwrap()["value"], "-7");
}

/// The log decides that content is sealed. A hub that holds no key refuses a write in the clear
/// as a person's host does, and a person's host that holds no key of the epoch writes nothing.
#[test]
fn a_list_that_keeps_content_takes_and_sends_no_write_in_the_clear() {
    let none = list(0);
    let one = list(1);

    // In a list that has begun no epoch a write is in the clear, and a list that has begun
    // one refuses it, at the hub and at a member.
    let mut alice_in_the_clear = host_in(&none, Some((1, 1)), &[]);
    let in_the_clear = send(&mut alice_in_the_clear, &titled(1, 1000, "buy milk"));
    assert_eq!(text_of(&in_the_clear), "buy milk");
    for host in [
        host_in(&one, None, &[]),
        host_in(&one, Some((2, 2)), &[(1, 5)]),
    ] {
        let mut host = host;
        let reason = host
            .admit("server", "todo.write", &in_the_clear)
            .unwrap_err();
        assert!(reason.contains("not sealed"), "{reason}");
    }

    // A host in the list that holds no key of its epoch does not fall back to the clear.
    let mut keyless_alice = host_in(&one, Some((1, 1)), &[]);
    let reason = try_send(&mut keyless_alice, &titled(1, 1000, "buy milk")).unwrap_err();
    assert!(reason.contains("holds no key of epoch 1"), "{reason}");
}

#[test]
fn sealed_content_under_another_key_or_epoch_is_refused() {
    let two = list(2);
    let one = &two[..two.len() - 1];
    let mut alice = host_in(&two, Some((1, 1)), &[(2, 6)]);
    let wire = send(&mut alice, &titled(1, 1000, "buy milk"));

    let mut without_it = host_in(&two, Some((2, 2)), &[(1, 5)]);
    let reason = without_it.admit("server", "todo.write", &wire).unwrap_err();
    assert!(reason.contains("no key of epoch 2"), "{reason}");

    let mut another_key = host_in(&two, Some((2, 2)), &[(2, 7)]);
    let reason = another_key
        .admit("server", "todo.write", &wire)
        .unwrap_err();
    assert!(reason.contains("does not open"), "{reason}");

    let mut right_key = host_in(&two, Some((2, 2)), &[(1, 5), (2, 6)]);
    assert!(right_key.admit("server", "todo.write", &wire).is_ok());

    // A host whose log has not begun epoch 2 does not take what is sealed under it: the operation
    // names the entry that begins it, which the host does not hold, and neither does a hub that
    // holds no key.
    let mut behind = host_in(one, Some((2, 2)), &[(1, 5), (2, 6)]);
    let reason = behind.admit("server", "todo.write", &wire).unwrap_err();
    assert!(reason.contains("does not hold as it is"), "{reason}");
    let mut behind_hub = host_in(one, None, &[]);
    assert!(behind_hub.admit("alice", "todo.write", &wire).is_err());
}

#[test]
fn a_write_seals_under_the_epoch_the_log_is_in_and_the_older_ones_still_open() {
    let two = list(2);
    let one = &two[..two.len() - 1];
    let mut alice = host_in(one, Some((1, 1)), &[(1, 5)]);
    let old = send(&mut alice, &titled(1, 1000, "old"));

    // Epoch 2 begins: alice's log has the entry and, until she holds its key, she writes nothing.
    alice.append(two.last().unwrap()).unwrap();
    assert_eq!(alice.epoch(), 2);
    let reason = try_send(&mut alice, &titled(1, 2000, "new")).unwrap_err();
    assert!(reason.contains("holds no key of epoch 2"), "{reason}");
    alice.set_epoch_key(2, [6; 32]);
    let new = send(&mut alice, &titled(1, 2000, "new"));
    assert_eq!(alice.epochs(), vec![1, 2]);

    let mut bob = host_in(&two, Some((2, 2)), &[(1, 5), (2, 6)]);
    for (wire, text) in [(&old, "old"), (&new, "new")] {
        let told = bob.admit("server", "todo.write", wire).unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&told).unwrap()["text"], text);
    }
    // What was sealed before is sent again as it was, on a resync.
    let told = alice.admit("server", "todo.write", &old).unwrap().unwrap();
    assert_eq!(
        text_of(&try_send(&mut alice, &told).unwrap().unwrap()),
        text_of(&old),
        "one write is one set of bytes"
    );
}

#[test]
fn a_key_is_given_to_a_member_and_to_no_one_else() {
    let one = list(1);
    let alice = host_in(&one, Some((1, 1)), &[(1, 5)]);
    let bob = host_in(&one, Some((2, 2)), &[]);
    let carol = host_in(&one, Some((3, 3)), &[]);
    let grant = alice.grant_key(1, &person(2).public()).unwrap();
    assert!(
        !grant.contains(&sce_rust_trust::encoding::encode(&[5u8; 32])),
        "the key is not in the grant"
    );

    assert_eq!(carol.receive_grant(&grant), Err(GrantError::NotForThisHost));
    assert_eq!(bob.receive_grant(&grant), Ok(1));
    assert_eq!(bob.epochs(), vec![1]);

    // What alice sealed under it, bob opens.
    let mut alice = alice;
    let mut bob = bob;
    let wire = send(&mut alice, &titled(1, 1000, "buy milk"));
    assert!(bob.admit("server", "todo.write", &wire).is_ok());

    // A grant that is changed, for another list, or from a key nobody knows is not taken.
    let changed = grant.replace("\"epoch\":1", "\"epoch\":4");
    assert_eq!(
        host_in(&one, Some((2, 2)), &[]).receive_grant(&changed),
        Err(GrantError::CannotOpen)
    );
    let other_list = grant.replace("\"home\"", "\"work\"");
    assert_eq!(
        host_in(&one, Some((2, 2)), &[]).receive_grant(&other_list),
        Err(GrantError::WrongCalendar)
    );
    let stranger = host_in(&one, Some((66, 66)), &[(1, 5)]);
    let from_a_stranger = stranger.grant_key(1, &person(2).public()).unwrap();
    assert_eq!(
        host_in(&one, Some((2, 2)), &[]).receive_grant(&from_a_stranger),
        Err(GrantError::UnknownGiver)
    );
    assert_eq!(
        host_in(&one, Some((2, 2)), &[]).receive_grant("not json"),
        Err(GrantError::Malformed)
    );
    assert_eq!(
        host_in(&one, None, &[(1, 5)]).grant_key(1, &person(2).public()),
        None
    );
}

#[test]
fn what_a_host_keeps_comes_back_from_its_store_and_still_opens() {
    let one = list(1);
    let mut alice = host_in(&one, Some((1, 1)), &[(1, 5)]);
    let mut hub = host_in(&one, None, &[]);
    let wire = send(&mut alice, &titled(1, 1000, "buy milk"));
    let held = hub.admit("alice", "todo.write", &wire).unwrap().unwrap();
    let text = hub.to_text();
    assert!(!text.contains("buy milk"));

    let mut restarted = host_in(&one, None, &[]);
    assert_eq!(restarted.restore_from_text(&text), 1);
    assert_eq!(text_of(&send(&mut restarted, &held)), text_of(&wire));

    // A person's host that kept the key reads the same text back, and holds the write as its
    // machine holds it.
    let mut bob = host_in(&one, Some((2, 2)), &[(1, 5)]);
    let told = bob.admit("server", "todo.write", &wire).unwrap().unwrap();
    let text = bob.to_text();
    let mut restarted = host_in(&one, Some((2, 2)), &[(1, 5)]);
    assert_eq!(restarted.restore_from_text(&text), 1);
    assert_eq!(text_of(&send(&mut restarted, &told)), text_of(&wire));
}

/// A member is taken off, and an administrator begins a new epoch for the members that are left:
/// the log says the epoch began, what is written from then on is sealed under a key the removed
/// member was never given, and they open none of it; what was sealed before opens for whoever
/// held its key, as it did.
#[test]
fn a_member_taken_off_opens_nothing_written_after_the_next_epoch() {
    let one = list(1);
    let mut alice = host_in(&one, Some((1, 1)), &[(1, 5)]);
    let mut bob = host_in(&one, Some((2, 2)), &[(1, 5)]);
    let mut carol = host_in(&one, Some((3, 3)), &[(1, 5)]);
    let before = send(&mut alice, &titled(1, 1000, "before"));

    // The administrator's host holds the list's log and the key it knows, takes bob off and
    // begins epoch 2 for who is left.
    let administrator = host_in(&one, None, &[(1, 5)]);
    let off = administrator
        .draft(Body::Grant {
            node: 2,
            role: 0,
            key: person(2).public(),
            final_seq: Some(0),
        })
        .sign(&admin())
        .finish();
    administrator.append(&off).unwrap();
    let left: Vec<_> = administrator
        .current_members()
        .into_iter()
        .map(|(_, key)| key)
        .collect();
    assert_eq!(left, vec![person(1).public(), person(3).public()]);
    let rotation = administrator.rotate_epoch(&admin(), &left).unwrap();
    assert_eq!(rotation.epoch, 2);
    assert_eq!(rotation.grants.len(), 2);
    assert_eq!(administrator.epochs(), vec![1, 2]);
    assert_eq!(administrator.epoch(), 2, "the log began it");

    // Every host is shown the two entries (a removed member's too: the log is no secret from them).
    for host in [&alice, &bob, &carol] {
        assert_eq!(host.append(&off), Ok(true));
        assert_eq!(host.append(&rotation.entry), Ok(true));
        assert_eq!(host.epoch(), 2);
    }

    // Carol has the entry and not yet the key: she writes nothing, in the clear or sealed under
    // the old one.
    let reason = try_send(&mut carol, &titled(3, 2000, "early")).unwrap_err();
    assert!(
        reason.contains("holds no key of epoch 2") || reason.contains("may not write"),
        "{reason}"
    );

    for (key, grant) in &rotation.grants {
        let who = if *key == person(1).public() {
            &alice
        } else {
            &carol
        };
        assert_eq!(who.receive_grant(grant), Ok(2));
    }
    // The grants are for those two and no one else; bob cannot use either.
    for (_, grant) in &rotation.grants {
        assert_eq!(bob.receive_grant(grant), Err(GrantError::NotForThisHost));
    }
    assert_eq!(bob.epochs(), vec![1]);

    // From now on alice seals under epoch 2; carol opens it and bob does not.
    let after = send(&mut alice, &titled(1, 2000, "after"));
    let told = carol
        .admit("server", "todo.write", &after)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&told).unwrap()["text"],
        "after"
    );
    let reason = bob.admit("server", "todo.write", &after).unwrap_err();
    assert!(reason.contains("no key of epoch 2"), "{reason}");

    // What was sealed before opens for the keys that opened it: the new key is no part of it.
    assert!(carol.admit("server", "todo.write", &before).is_ok());
    assert!(bob.admit("server", "todo.write", &before).is_ok());

    // Only an administrator begins an epoch, and a key that is held is not replaced.
    assert_eq!(
        administrator.rotate_epoch(&person(1), &left),
        Err(RotateError::NotAnAdministrator)
    );
    let held_ahead = host_in(&one, None, &[(2, 6)]);
    assert_eq!(
        held_ahead.rotate_epoch(&admin(), &left),
        Err(RotateError::EpochHeld),
        "a host that holds a key of the next epoch has not been shown its entry"
    );
    let other = host_in(&one, None, &[(1, 5)]);
    let clash = other.rotate_epoch(&admin(), &left).unwrap();
    assert_eq!(clash.epoch, 2);
    let one_held_already = host_in(&one, Some((1, 1)), &[(2, 7)]);
    assert_eq!(
        one_held_already.receive_grant(&clash.grants[0].1),
        Err(GrantError::EpochTaken)
    );
}
