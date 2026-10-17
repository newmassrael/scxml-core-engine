// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What a host that does not trust the host beside it lets through. A write leaves with its
// author's signature and arrives at a host that holds the list's membership log: it is admitted
// when the author was a member who could write at the time they made it and the signature is
// true, and it reaches the machine as the machine wrote it. What anyone else does with it (a
// changed field, a key that is not the author's, a reader, a stranger, another list, a member
// taken off, a host shown two histories of the log) is refused, and what a hub relays it relays
// with the signature the member made.

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
/// each of `grants`, as `(node, role, key seed, from)`.
fn entries_of(list: &str, profile: Profile, grants: &[(u64, u8, u8, i64)]) -> Vec<Entry> {
    let scratch = Trust::open(list);
    let genesis = genesis_of(list, profile);
    scratch.append(&genesis).unwrap();
    let mut entries = vec![genesis];
    for (node, role, seed, from) in grants {
        let entry = scratch
            .draft(Body::Grant {
                node: *node,
                role: *role,
                key: person(*seed).public(),
                from: *from,
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
        &[(1, 2, 1, 0), (2, 2, 2, 0), (3, 1, 3, 0)],
    )
}

fn host_with(entries: &[Entry], own: Option<(u64, Identity)>) -> Trust {
    let trust = Trust::open(CALENDAR);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if let Some((node, identity)) = own {
        trust.set_identity(node, identity);
        trust.set_clock(|| 2000);
    }
    trust
}

/// A host of the standard list, for the person `own` when there is one.
fn host(own: Option<(u64, Identity)>) -> Trust {
    host_with(&standard(), own)
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
    let work = entries_of("work", Profile::Untrusted, &[(1, 2, 1, 0)]);
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
fn a_hub_hands_on_what_a_member_wrote_with_the_signature_it_arrived_with() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let mut bob = host(Some((2, person(2))));
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    // The hub's machine sends it on to bob as the write it holds.
    let relayed = sent(&mut hub, "todo.write", &for_the_machine);
    assert!(bob.admit("server", "todo.write", &relayed).is_ok());
    // A write it never saw it cannot send on: it holds no signature for it and signs nothing
    // of its own, and it says so instead of sending something else.
    let reason = send(&mut hub, "todo.write", &write(1, 99)).unwrap_err();
    assert!(reason.contains("signs none in their name"), "{reason}");
}

#[test]
fn a_member_is_taken_off_by_a_later_entry_and_what_was_admitted_stays_known() {
    let mut alice = host(Some((1, person(1))));
    let mut entries = standard();
    let mut hub = host_with(&entries, None);
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    assert!(hub.admit("alice", "todo.write", &sealed).is_ok());

    let scratch = host_with(&entries, None);
    let off = scratch
        .draft(Body::Grant {
            node: 1,
            role: 0,
            key: person(1).public(),
            from: 1500,
        })
        .sign(&admin())
        .finish();
    assert_eq!(hub.append(&off), Ok(true));
    assert_eq!(
        hub.append(&off),
        Ok(false),
        "an entry held is taken in once"
    );
    entries.push(off);
    assert_eq!(hub.role(1), 0);
    // What was made before the time it takes effect from is still theirs.
    assert!(hub.admit("alice", "todo.write", &sealed).is_ok());
    let after = sent(&mut alice, "todo.write", &write_at(1, 10, 1600));
    assert!(hub.admit("alice", "todo.write", &after).is_err());
}

#[test]
fn an_entry_the_administrators_did_not_sign_is_not_taken_in() {
    let trust = Trust::open(CALENDAR);
    trust
        .append(&genesis_of(CALENDAR, Profile::Untrusted))
        .unwrap();
    let by_mallory = trust
        .draft(Body::Grant {
            node: 1,
            role: 3,
            key: person(1).public(),
            from: 0,
        })
        .sign(&person(66))
        .finish();
    assert_eq!(
        trust.append(&by_mallory),
        Err(LogError::NotEnoughSignatures { needed: 1, had: 0 })
    );
    let mut edited = trust
        .draft(Body::Grant {
            node: 1,
            role: 2,
            key: person(1).public(),
            from: 0,
        })
        .sign(&admin())
        .finish();
    edited.body = Body::Grant {
        node: 1,
        role: 3,
        key: person(1).public(),
        from: 0,
    };
    assert_eq!(trust.append(&edited), Err(LogError::BadSignature));
    assert_eq!(trust.role(1), 0);
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
            from: 0,
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
    entries.push(
        scratch
            .draft(Body::Grant {
                node: 3,
                role: 0,
                key: person(3).public(),
                from: 0,
            })
            .sign(&admin())
            .finish(),
    );
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
fn the_signatures_a_host_relays_with_survive_a_restart_when_they_are_still_true() {
    let mut alice = host(Some((1, person(1))));
    let mut hub = host(None);
    let sealed = sent(&mut alice, "todo.write", &write(1, 10));
    let for_the_machine = hub.admit("alice", "todo.write", &sealed).unwrap().unwrap();
    let kept = hub.signatures_to_text();

    let mut restarted = host(None);
    assert_eq!(restarted.restore_signatures_from_text(&kept), 1);
    assert!(matches!(
        send(&mut restarted, "todo.write", &for_the_machine),
        Ok(Some(_))
    ));

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
    let trusted = entries_of(CALENDAR, Profile::TrustedHub, &[(1, 2, 1, 0)]);
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

/// A member taken off from a time is still the writer of what they made before it, and not of
/// what they made at or after it.
#[test]
fn a_member_taken_off_from_a_time_still_wrote_what_came_before() {
    let entries = entries_of(
        CALENDAR,
        Profile::Untrusted,
        &[(1, 2, 1, 0), (1, 0, 1, 1500)],
    );
    let mut alice = host(Some((1, person(1))));
    let mut hub = host_with(&entries, None);
    let before = sent(&mut alice, "todo.write", &write_at(1, 10, 1000));
    let after = sent(&mut alice, "todo.write", &write_at(1, 10, 1600));
    assert!(hub.admit("x", "todo.write", &before).is_ok());
    let reason = hub.admit("x", "todo.write", &after).unwrap_err();
    assert!(reason.contains("may not write"), "{reason}");
    assert_eq!(hub.role(1), 0, "now they are off");
}

/// A new key judges what is made from its time, and the old key what was made before it.
#[test]
fn a_replaced_key_judges_only_what_is_made_from_its_time() {
    let entries = entries_of(
        CALENDAR,
        Profile::Untrusted,
        &[(1, 2, 1, 0), (1, 2, 11, 2000)],
    );
    let mut hub = host_with(&entries, None);
    let mut old = host(Some((1, person(1))));
    let mut new = host(Some((1, person(11))));

    let old_then = sent(&mut old, "todo.write", &write_at(1, 1, 1000));
    let old_later = sent(&mut old, "todo.write", &write_at(1, 1, 3000));
    let new_then = sent(&mut new, "todo.write", &write_at(1, 1, 1000));
    let new_later = sent(&mut new, "todo.write", &write_at(1, 1, 3000));
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

    let sealed = sent(&mut alice, "todo.supersede", &supersede(1));
    let admitted = hub
        .admit("alice", "todo.supersede", &sealed)
        .unwrap()
        .unwrap();
    let admitted: Value = serde_json::from_str(&admitted).unwrap();
    assert!(admitted.get("sig").is_none() && admitted.get("at").is_none());
    assert_eq!(admitted["by"], "1");

    // The hub's machine sends the tombstone again as itself, on a resync.
    let relayed = sent(&mut hub, "todo.supersede", &supersede(100));
    let relayed_value: Value = serde_json::from_str(&relayed).unwrap();
    assert_eq!(relayed_value["by"], "1", "who really wrote over it");
    assert_eq!(relayed_value["at"], "2000");
    assert!(bob.admit("server", "todo.supersede", &relayed).is_ok());

    // A note nobody made is not sent on, and one the host's person did not make is not signed.
    let mut other = host(None);
    assert!(send(&mut other, "todo.supersede", &supersede(100)).is_err());
    let mut carol = host(Some((3, person(3))));
    assert!(send(&mut carol, "todo.supersede", &supersede(1)).is_err());
}

/// A member taken off writes over a value no more than they write a value: a note made at a
/// time after they were taken off is refused, and one made before it is not.
#[test]
fn a_member_taken_off_writes_no_notes_after_it() {
    let entries = entries_of(
        CALENDAR,
        Profile::Untrusted,
        &[(1, 2, 1, 0), (1, 0, 1, 1500)],
    );
    let mut hub = host_with(&entries, None);
    for (clock, taken_in) in [(1200, true), (2000, false)] {
        let mut alice = host_with(&entries, Some((1, person(1))));
        alice.set_clock(move || clock);
        let sealed = sent(&mut alice, "todo.supersede", &supersede(1));
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
        &entries_of(CALENDAR, Profile::Untrusted, &[(1, 0, 1, 0)]),
        None,
    );
    assert_eq!(changed.restore_signatures_from_text(&text), 0);
}

/// The roster a hub's machine is told keeps a member who could write at some time a writer,
/// where a person's machine is told the role they have now.
#[test]
fn the_hub_machine_is_told_who_ever_wrote() {
    let entries = entries_of(
        CALENDAR,
        Profile::Untrusted,
        &[(1, 2, 1, 0), (1, 0, 1, 1500), (3, 1, 3, 0)],
    );
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
