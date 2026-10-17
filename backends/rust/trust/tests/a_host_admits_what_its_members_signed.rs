// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a host that does not trust the host beside it lets through. A write leaves with its
// author's signature and arrives at a host that holds the administrator's certificates: it is
// admitted when the author is a member who may write and the signature is true, and it reaches
// the machine as the machine wrote it. What anyone else does with it (a changed field, a key
// that is not the author's, a reader, a stranger, another list, a member taken off) is
// refused, and what a hub relays it relays with the signature the member made.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::{Certificate, Identity, Trust, TrustError};
use serde_json::{json, Value};

const CALENDAR: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn certificate(node: u64, role: u8, who: &Identity, serial: u64) -> Certificate {
    certificate_from(node, role, who, serial, 0)
}

fn certificate_from(node: u64, role: u8, who: &Identity, serial: u64, from: i64) -> Certificate {
    Certificate::issue(&admin(), CALENDAR, node, role, who.public(), serial, from)
}

/// A host of the list that trusts `admin()` and holds the certificates of alice (node 1,
/// writer), bob (node 2, writer) and carol (node 3, reader).
fn host(own: Option<(u64, Identity)>) -> Trust {
    let trust = Trust::new(CALENDAR, &[admin().public()]);
    for (node, role, seed) in [(1, 2, 1), (2, 2, 2), (3, 1, 3)] {
        trust
            .accept(&certificate(node, role, &person(seed), 1))
            .unwrap();
    }
    if let Some((node, identity)) = own {
        trust.set_identity(node, identity);
        trust.set_clock(|| 2000);
    }
    trust
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

fn send(host: &mut Trust, event: &str, data: &str) -> Option<String> {
    host.seal("#server", event, data)
}

#[test]
fn a_write_a_member_signed_is_admitted_and_arrives_as_the_machine_wrote_it() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    assert!(sealed.contains("\"sig\""), "{sealed}");
    let admitted = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    let wrote: Value = serde_json::from_str(&write(1, 10)).unwrap();
    assert_eq!(
        admitted, wrote,
        "the machine is told what it was sent, signature out"
    );
}

#[test]
fn what_is_not_a_signed_event_goes_through_untouched() {
    let mut hub = host(None);
    assert_eq!(hub.admit("alice", "todo.node", "{}"), Ok(None));
    assert_eq!(send(&mut hub, "todo.node", "{}"), None);
}

#[test]
fn a_write_with_no_signature_or_a_changed_field_or_another_key_is_refused() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let unsigned = write(1, 10);
    assert!(hub.admit("alice", "todo.write", &unsigned).is_err());

    let sealed = send(&mut alice, "todo.write", &unsigned).unwrap();
    let edited = sealed.replace("\"10\"", "\"11\"");
    assert_ne!(edited, sealed);
    let reason = hub.admit("alice", "todo.write", &edited).unwrap_err();
    assert!(reason.contains("not node 1's"), "{reason}");

    // Bob's host does not sign in alice's name, and a host that holds another key for node 1
    // does, which the hub's certificate for node 1 does not verify.
    let mut bob = host(Some((2, person(2))));
    assert_eq!(send(&mut bob, "todo.write", &write(1, 10)), None);
    let mut forger = Trust::new(CALENDAR, &[admin().public()]);
    forger.set_identity(1, person(5));
    let forged = send(&mut forger, "todo.write", &write(1, 10)).unwrap();
    assert!(hub.admit("x", "todo.write", &forged).is_err());
}

#[test]
fn a_reader_and_a_stranger_may_not_write_whatever_they_sign() {
    let mut hub = host(None);
    for (node, seed) in [(3u64, 3u8), (4, 4)] {
        let mut copy = Trust::new(CALENDAR, &[admin().public()]);
        copy.set_identity(node, person(seed));
        let sealed = send(&mut copy, "todo.write", &write(node, 1)).unwrap();
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
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    let mut other = Trust::new("work", &[admin().public()]);
    let cert = Certificate::issue(&admin(), "work", 1, 2, person(1).public(), 1, 0);
    other.accept(&cert).unwrap();
    assert!(other.admit("x", "todo.write", &sealed).is_err());
}

#[test]
fn a_hub_hands_on_what_a_member_wrote_with_the_signature_it_arrived_with() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let mut bob = host(Some((2, person(2))));
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    // The hub's machine sends it on to bob as the write it holds.
    let relayed = send(&mut hub, "todo.write", &for_the_machine).unwrap();
    assert!(bob.admit("server", "todo.write", &relayed).is_ok());
    // A write it never saw it cannot send on: it holds no signature for it and signs
    // nothing of its own.
    assert_eq!(send(&mut hub, "todo.write", &write(1, 99)), None);
}

#[test]
fn a_member_is_taken_off_by_a_later_certificate_and_an_older_one_changes_nothing() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    assert!(hub.admit("alice", "todo.write", &sealed).is_ok());

    assert_eq!(
        hub.accept(&certificate(1, 0, &person(1), 1)),
        Err(TrustError::Stale { held: 1 })
    );
    assert!(hub.admit("alice", "todo.write", &sealed).is_ok());

    hub.accept(&certificate(1, 0, &person(1), 2)).unwrap();
    assert_eq!(hub.role(1), 0);
    assert!(hub.admit("alice", "todo.write", &sealed).is_err());
}

#[test]
fn a_certificate_the_host_does_not_trust_is_not_taken_in() {
    let trust = Trust::new(CALENDAR, &[admin().public()]);
    let member = person(1);
    let mallory = person(66);
    let by_mallory = Certificate::issue(&mallory, CALENDAR, 1, 3, member.public(), 1, 0);
    assert_eq!(trust.accept(&by_mallory), Err(TrustError::UnknownIssuer));
    let other_list = Certificate::issue(&admin(), "work", 1, 2, member.public(), 1, 0);
    assert_eq!(trust.accept(&other_list), Err(TrustError::WrongCalendar));
    let edited = Certificate {
        role: 3,
        ..certificate(1, 2, &member, 1)
    };
    assert_eq!(trust.accept(&edited), Err(TrustError::BadSignature));
    assert_eq!(trust.role(1), 0);
}

#[test]
fn the_roster_the_certificates_make_is_what_the_machine_is_told() {
    let hub = host(None);
    hub.accept(&certificate(3, 0, &person(3), 2)).unwrap();
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
fn the_signatures_a_host_relays_with_survive_a_restart_when_they_are_still_true() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    let kept = hub.signatures();
    assert_eq!(kept.len(), 1);

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures(&kept), 1);
    assert!(send(&mut restarted, "todo.write", &for_the_machine).is_some());

    let mut tampered = kept.clone();
    tampered[0].1[0] ^= 1;
    let mut again = host(None);
    assert_eq!(again.restore_signatures(&tampered), 0);
    assert_eq!(send(&mut again, "todo.write", &for_the_machine), None);
}

#[test]
fn the_signatures_a_host_keeps_in_its_store_read_back_as_text() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = send(&mut alice, "todo.write", &write(1, 10)).unwrap();
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    hub.attest_floor(&admin(), 30_000, 7).unwrap();
    let text = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&text), 2);
    assert!(send(&mut restarted, "todo.write", &for_the_machine).is_some());
    assert!(send(
        &mut restarted,
        "todo.floor",
        r#"{"before":"30000","hash":"7"}"#
    )
    .is_some());

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
    assert_eq!(send(&mut hub, "todo.floor", floor), None);
    assert!(person_host.admit("server", "todo.floor", floor).is_err());

    hub.attest_floor(&admin(), 30_000, 7).unwrap();
    let sealed = send(&mut hub, "todo.floor", floor).unwrap();
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
        let moved = sealed.replace(floor, other);
        let mut value: Value = serde_json::from_str(other).unwrap();
        value["sig"] = serde_json::from_str::<Value>(&sealed).unwrap()["sig"].clone();
        let reason = person_host
            .admit("server", "todo.floor", &value.to_string())
            .unwrap_err();
        assert!(
            reason.contains("no administrator signed"),
            "{moved} {reason}"
        );
    }
}

#[test]
fn only_a_key_the_host_trusts_attests_a_floor() {
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

/// A member taken off from a time is still the writer of what they made before it, and not of
/// what they made at or after it, whatever order the host received the certificates in.
#[test]
fn a_member_taken_off_from_a_time_still_wrote_what_came_before() {
    for reversed in [false, true] {
        let mut alice = host(Some((1, person(1))));
        let mut hub = Trust::new(CALENDAR, &[admin().public()]);
        let mut certificates = vec![
            certificate_from(1, 2, &person(1), 1, 0),
            certificate_from(1, 0, &person(1), 2, 1500),
        ];
        if reversed {
            certificates.reverse();
        }
        for certificate in &certificates {
            hub.accept(certificate).unwrap();
        }
        let before = send(&mut alice, "todo.write", &write_at(1, 10, 1000)).unwrap();
        let after = send(&mut alice, "todo.write", &write_at(1, 10, 1600)).unwrap();
        assert!(hub.admit("x", "todo.write", &before).is_ok(), "{reversed}");
        let reason = hub.admit("x", "todo.write", &after).unwrap_err();
        assert!(reason.contains("may not write"), "{reason}");
        assert_eq!(hub.role(1), 0, "now they are off");
    }
}

/// A new key judges what is made from its time, and the old key what was made before it.
#[test]
fn a_replaced_key_judges_only_what_is_made_from_its_time() {
    let mut hub = Trust::new(CALENDAR, &[admin().public()]);
    hub.accept(&certificate_from(1, 2, &person(1), 1, 0))
        .unwrap();
    hub.accept(&certificate_from(1, 2, &person(11), 2, 2000))
        .unwrap();
    let mut old = Trust::new(CALENDAR, &[admin().public()]);
    old.set_identity(1, person(1));
    let mut new = Trust::new(CALENDAR, &[admin().public()]);
    new.set_identity(1, person(11));

    let old_then = send(&mut old, "todo.write", &write_at(1, 1, 1000)).unwrap();
    let old_later = send(&mut old, "todo.write", &write_at(1, 1, 3000)).unwrap();
    let new_then = send(&mut new, "todo.write", &write_at(1, 1, 1000)).unwrap();
    let new_later = send(&mut new, "todo.write", &write_at(1, 1, 3000)).unwrap();
    assert!(hub.admit("x", "todo.write", &old_then).is_ok());
    assert!(hub.admit("x", "todo.write", &old_later).is_err());
    assert!(hub.admit("x", "todo.write", &new_then).is_err());
    assert!(hub.admit("x", "todo.write", &new_later).is_ok());
}

/// What a person's copy sends when it wrote over a value is signed with the time it was made
/// at; the hub takes the note in without the host's fields, and when its machine sends the
/// tombstone again (as itself, on a resync) the host puts back who wrote over it and their
/// signature, so that a host that trusts nobody takes it from the hub.
#[test]
fn a_note_that_a_write_was_written_over_keeps_who_wrote_over_it() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let mut bob = host(Some((2, person(2))));

    let sealed = send(&mut alice, "todo.supersede", &supersede(1)).unwrap();
    let admitted = hub
        .admit("alice", "todo.supersede", &sealed)
        .unwrap()
        .unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    assert!(admitted.get("sig").is_none() && admitted.get("at").is_none());
    assert_eq!(admitted["by"], "1");

    // The hub's machine sends the tombstone again as itself, on a resync.
    let relayed = send(&mut hub, "todo.supersede", &supersede(100)).unwrap();
    let relayed_value: Value = serde_json::from_str(&relayed).unwrap();
    assert_eq!(relayed_value["by"], "1", "who really wrote over it");
    assert_eq!(relayed_value["at"], "2000");
    assert!(bob.admit("server", "todo.supersede", &relayed).is_ok());

    // A note nobody made is not sent on, and one the host's person did not make is not signed.
    let mut other = host(None);
    assert_eq!(send(&mut other, "todo.supersede", &supersede(100)), None);
    let mut carol = host(Some((3, person(3))));
    assert_eq!(send(&mut carol, "todo.supersede", &supersede(1)), None);
}

/// A member taken off writes over a value no more than they write a value: a note made at a
/// time after they were taken off is refused, and one made before it is not.
#[test]
fn a_member_taken_off_writes_no_notes_after_it() {
    let mut hub = host(None);
    hub.accept(&certificate_from(1, 0, &person(1), 2, 1500))
        .unwrap();
    for (clock, taken_in) in [(1200, true), (2000, false)] {
        let mut alice = Trust::new(CALENDAR, &[admin().public()]);
        alice.set_identity(1, person(1));
        alice.set_clock(move || clock);
        let sealed = send(&mut alice, "todo.supersede", &supersede(1)).unwrap();
        assert_eq!(
            hub.admit("alice", "todo.supersede", &sealed).is_ok(),
            taken_in,
            "{clock}"
        );
    }
}

/// What a host keeps of the notes comes back from its store when it is still true.
#[test]
fn the_notes_a_host_keeps_come_back_from_its_store() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = send(&mut alice, "todo.supersede", &supersede(1)).unwrap();
    hub.admit("alice", "todo.supersede", &sealed).unwrap();
    let text = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&text), 1);
    assert!(send(&mut restarted, "todo.supersede", &supersede(100)).is_some());

    let changed = host(None);
    changed
        .accept(&certificate_from(1, 0, &person(1), 2, 0))
        .unwrap();
    assert_eq!(changed.restore_signatures_from_text(&text), 0);
}

/// The roster a hub's machine is told keeps a member who could write at some time a writer,
/// where a person's machine is told the role they have now.
#[test]
fn the_hub_machine_is_told_who_ever_wrote() {
    let hub = host(None);
    hub.accept(&certificate_from(1, 0, &person(1), 2, 1500))
        .unwrap();
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
