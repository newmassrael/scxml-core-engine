// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A list whose members hold a key the hub does not. A write leaves its author's host with its
// number and text sealed under the key of the latest epoch, signed as it travels; the hub's
// host, which holds no key, takes it as it is and relays it as it is, and the host of another
// member opens it and tells its machine the write as its author made it. What a host that holds
// keys will not take is a write that is not sealed, or sealed under an epoch it holds no key of.
// A key is given to a member sealed so that only that member opens it, and it comes from a key
// the list's members or administrators hold.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::{Certificate, GrantError, Identity, RotateError, Trust};
use serde_json::{json, Value};

const CALENDAR: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

/// A host of the list holding the certificates of alice (node 1), bob (node 2) and carol
/// (node 3, a reader), for the person `own` and with the keys `epochs`.
fn host(own: Option<(u64, u8)>, epochs: &[(u32, u8)]) -> Trust {
    let trust = Trust::new(CALENDAR, &[admin().public()]);
    for (node, role) in [(1, 2), (2, 2), (3, 1)] {
        trust
            .accept(&Certificate::issue(
                &admin(),
                CALENDAR,
                node,
                role,
                person(node as u8).public(),
                1,
                0,
            ))
            .unwrap();
    }
    if let Some((node, seed)) = own {
        trust.set_identity(node, person(seed));
    }
    for (epoch, fill) in epochs {
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

fn send(host: &mut Trust, data: &str) -> Option<String> {
    host.seal("#server", "todo.write", data)
}

fn text_of(data: &str) -> String {
    serde_json::from_str::<Value>(data).unwrap()["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_write_leaves_sealed_and_a_member_reads_it_where_the_hub_cannot() {
    let mut alice = host(Some((1, 1)), &[(1, 5)]);
    let mut hub = host(None, &[]);
    let mut bob = host(Some((2, 2)), &[(1, 5)]);

    let wire = send(&mut alice, &titled(1, 1000, "buy milk")).unwrap();
    assert!(text_of(&wire).starts_with('~'), "{wire}");
    assert!(!wire.contains("buy milk"));
    let on_wire: Value = serde_json::from_str(&wire).unwrap();
    assert_eq!(on_wire["value"], "0");

    // The hub's host holds no key: it admits what it can verify and its machine holds the
    // sealed text, which it sends on as it is.
    let held_by_the_hub = hub.admit("alice", "todo.write", &wire).unwrap().unwrap();
    assert_eq!(text_of(&held_by_the_hub), text_of(&wire));
    let relayed = send(&mut hub, &held_by_the_hub).unwrap();
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
    let wire = send(&mut alice, &numbered(1, 1001, -7)).unwrap();
    assert_eq!(text_of(&wire).chars().next(), Some('~'));
    assert_eq!(serde_json::from_str::<Value>(&wire).unwrap()["value"], "0");
    let told = bob.admit("server", "todo.write", &wire).unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&told).unwrap()["value"], "-7");
}

#[test]
fn a_host_that_keeps_content_takes_no_write_in_the_clear() {
    let mut keyless_alice = host(Some((1, 1)), &[]);
    let mut bob = host(Some((2, 2)), &[(1, 5)]);
    let in_the_clear = send(&mut keyless_alice, &titled(1, 1000, "buy milk")).unwrap();
    let reason = bob
        .admit("server", "todo.write", &in_the_clear)
        .unwrap_err();
    assert!(reason.contains("not sealed"), "{reason}");
}

#[test]
fn sealed_content_under_another_key_or_epoch_is_refused() {
    let mut alice = host(Some((1, 1)), &[(2, 6)]);
    let wire = send(&mut alice, &titled(1, 1000, "buy milk")).unwrap();

    let mut without_it = host(Some((2, 2)), &[(1, 5)]);
    let reason = without_it.admit("server", "todo.write", &wire).unwrap_err();
    assert!(reason.contains("no key of epoch 2"), "{reason}");

    let mut another_key = host(Some((2, 2)), &[(2, 7)]);
    let reason = another_key
        .admit("server", "todo.write", &wire)
        .unwrap_err();
    assert!(reason.contains("does not open"), "{reason}");

    let mut right_key = host(Some((2, 2)), &[(1, 5), (2, 6)]);
    assert!(right_key.admit("server", "todo.write", &wire).is_ok());
}

#[test]
fn a_write_seals_under_the_latest_epoch_and_the_older_ones_still_open() {
    let mut alice = host(Some((1, 1)), &[(1, 5)]);
    let old = send(&mut alice, &titled(1, 1000, "old")).unwrap();
    alice.set_epoch_key(2, [6; 32]);
    let new = send(&mut alice, &titled(1, 2000, "new")).unwrap();
    assert_eq!(alice.epochs(), vec![1, 2]);

    let mut bob = host(Some((2, 2)), &[(1, 5), (2, 6)]);
    for (wire, text) in [(&old, "old"), (&new, "new")] {
        let told = bob.admit("server", "todo.write", wire).unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&told).unwrap()["text"], text);
    }
    // What was sealed before is sent again as it was, on a resync.
    let told = alice.admit("server", "todo.write", &old).unwrap().unwrap();
    assert_eq!(
        text_of(&send(&mut alice, &told).unwrap()),
        text_of(&old),
        "one write is one set of bytes"
    );
}

#[test]
fn a_key_is_given_to_a_member_and_to_no_one_else() {
    let alice = host(Some((1, 1)), &[(3, 5)]);
    let bob = host(Some((2, 2)), &[]);
    let carol = host(Some((3, 3)), &[]);
    let grant = alice.grant_key(3, &person(2).public()).unwrap();
    assert!(
        !grant.contains(&sce_rust_trust::encoding::encode(&[5u8; 32])),
        "the key is not in the grant"
    );

    assert_eq!(carol.receive_grant(&grant), Err(GrantError::NotForThisHost));
    assert_eq!(bob.receive_grant(&grant), Ok(3));
    assert_eq!(bob.epochs(), vec![3]);

    // What alice sealed under it, bob opens.
    let mut alice = alice;
    let mut bob = bob;
    let wire = send(&mut alice, &titled(1, 1000, "buy milk")).unwrap();
    assert!(bob.admit("server", "todo.write", &wire).is_ok());

    // A grant that is changed, for another list, or from a key nobody knows is not taken.
    let changed = grant.replace("\"epoch\":3", "\"epoch\":4");
    assert_eq!(
        host(Some((2, 2)), &[]).receive_grant(&changed),
        Err(GrantError::CannotOpen)
    );
    let other_list = grant.replace("\"home\"", "\"work\"");
    assert_eq!(
        host(Some((2, 2)), &[]).receive_grant(&other_list),
        Err(GrantError::WrongCalendar)
    );
    let stranger = Trust::new(CALENDAR, &[admin().public()]);
    stranger.set_identity(66, person(66));
    stranger.set_epoch_key(3, [5; 32]);
    let from_a_stranger = stranger.grant_key(3, &person(2).public()).unwrap();
    assert_eq!(
        host(Some((2, 2)), &[]).receive_grant(&from_a_stranger),
        Err(GrantError::UnknownGiver)
    );
    assert_eq!(
        host(Some((2, 2)), &[]).receive_grant("not json"),
        Err(GrantError::Malformed)
    );
    assert_eq!(host(None, &[]).grant_key(3, &person(2).public()), None);
}

#[test]
fn what_a_host_keeps_comes_back_from_its_store_and_still_opens() {
    let mut alice = host(Some((1, 1)), &[(1, 5)]);
    let mut hub = host(None, &[]);
    let wire = send(&mut alice, &titled(1, 1000, "buy milk")).unwrap();
    let held = hub.admit("alice", "todo.write", &wire).unwrap().unwrap();
    let text = hub.signatures_to_text();
    assert!(!text.contains("buy milk"));

    let mut restarted = host(None, &[]);
    assert_eq!(restarted.restore_signatures_from_text(&text), 1);
    assert_eq!(
        text_of(&send(&mut restarted, &held).unwrap()),
        text_of(&wire)
    );

    // A person's host that kept the key reads the same text back, and holds the write as its
    // machine holds it.
    let mut bob = host(Some((2, 2)), &[(1, 5)]);
    let told = bob.admit("server", "todo.write", &wire).unwrap().unwrap();
    let text = bob.signatures_to_text();
    let mut restarted = host(Some((2, 2)), &[(1, 5)]);
    assert_eq!(restarted.restore_signatures_from_text(&text), 1);
    assert_eq!(
        text_of(&send(&mut restarted, &told).unwrap()),
        text_of(&wire)
    );
}

/// A member is taken off, and an administrator begins a new epoch for the members that are left:
/// what is written from then on is sealed under a key the removed member was never given, and
/// they open none of it; what was sealed before opens for whoever held its key, as it did.
#[test]
fn a_member_taken_off_opens_nothing_written_after_the_next_epoch() {
    let mut alice = host(Some((1, 1)), &[(1, 5)]);
    let mut bob = host(Some((2, 2)), &[(1, 5)]);
    let mut carol = host(Some((3, 3)), &[(1, 5)]);
    let before = send(&mut alice, &titled(1, 1000, "before")).unwrap();

    // The administrator's host holds the list's certificates and the key it knows, takes bob off
    // and begins epoch 2 for who is left.
    let administrator = host(None, &[(1, 5)]);
    administrator
        .accept(&Certificate::issue(
            &admin(),
            CALENDAR,
            2,
            0,
            person(2).public(),
            2,
            1500,
        ))
        .unwrap();
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
    let after = send(&mut alice, &titled(1, 2000, "after")).unwrap();
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
    let other = host(None, &[(2, 6)]);
    let clash = other.rotate_epoch(&admin(), &left).unwrap();
    assert_eq!(clash.epoch, 3);
    let one_held_already = host(Some((1, 1)), &[(3, 7)]);
    let grant_of_3 = &clash.grants[0].1;
    assert_eq!(
        one_held_already.receive_grant(grant_of_3),
        Err(GrantError::EpochTaken)
    );
}
