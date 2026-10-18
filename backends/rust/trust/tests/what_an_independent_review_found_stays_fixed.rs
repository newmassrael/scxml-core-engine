// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// What a reviewer who did not write this code found when they tried to break it (claudedocs: the
// review of 2026-10-18), each as the reproduction they gave and the property it broke. They are kept
// here by what they were about, not by who found them: a stamp names one write and a device does not sign
// two contents under it; an entry nobody signed freezes no host; a device's conflicting operations are
// kept as evidence, which travels, is stored and is audited; what a host holds of a device does not
// depend on the order the operations came in, for a stamp that goes back and for the first epoch that
// begins to seal a list; and a replacement key's chain begins where the cutoff ended the old one.
//
// A device that holds a key and means harm is played by hand (an operation made with the key and the
// fields the test wants), since an honest host refuses to make what is made here.

use sce_rust_mesh::endpoint::Sealer;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::message::{self, Link, NO_OPERATION, SIGNATURE_FIELD};
use sce_rust_trust::{audit, encoding, Identity, TakeError, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";

fn key(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn admin() -> Identity {
    key(9)
}

/// The entries of a list in which node 1 writes with key 1.
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
    publish(
        &scratch,
        Body::Grant {
            node: 1,
            role: 2,
            key: key(1).public(),
            final_seq: None,
        },
    );
    scratch.entries()
}

fn publish(host: &Trust, body: Body) -> Entry {
    let entry = host.draft(body).sign(&admin()).finish();
    host.append(&entry).unwrap();
    entry
}

/// A host that holds the log `entries`; `own` makes it node 1's person.
fn host(entries: &[Entry], own: bool) -> Trust {
    let trust = Trust::invited(LIST, entries[0].hash());
    for entry in entries {
        trust.append(entry).unwrap();
    }
    if own {
        trust.set_identity(1, key(1));
    }
    trust
}

fn write_data(time: i64, counter: u64, text: &str) -> String {
    json!({"id": "1", "field": 1, "value": "0", "text": text,
           "wallTime": time.to_string(), "counter": counter.to_string(), "nodeId": "1"})
    .to_string()
}

/// The write as node 1's host sends it, through the host's own signing.
fn sealed(host: &mut Trust, time: i64, text: &str) -> Result<String, String> {
    host.seal("relay", "todo.write", &write_data(time, 0, text))
        .map(|sent| sent.expect("a write is sent"))
}

/// An operation of node 1 made by hand with `signer`: the number, what it names before it, the entry it
/// names (the tip of `host`), and the stamp and content of its write, as the device makes it.
fn forged(
    host: &Trust,
    signer: &Identity,
    seq: u64,
    prev: [u8; 32],
    time: i64,
    counter: u64,
    text: &str,
) -> (String, [u8; 32]) {
    let mut value: Value = serde_json::from_str(&write_data(time, counter, text)).unwrap();
    let signing = message::signing(LIST, "todo.write", &value)
        .unwrap()
        .unwrap();
    let (next, log_hash) = host.head();
    let link = Link {
        seq,
        prev,
        log: next - 1,
        log_hash,
    };
    let bytes = message::operation_bytes(&signing.message, &link);
    link.write_into(&mut value).unwrap();
    value[SIGNATURE_FIELD] = Value::String(encoding::encode(&signer.sign(&bytes)));
    (value.to_string(), message::operation_id(&bytes))
}

fn take(host: &Trust, data: &str) -> Result<sce_rust_trust::Taken, TakeError> {
    host.take_operation("todo.write", data)
}

/// What a host holds of every device: the operations it tells its machine, device by device.
fn held(host: &Trust) -> Vec<(String, String)> {
    host.all_operations()
}

#[test]
fn a_stamp_names_one_write_and_a_device_signs_no_second_content_under_it() {
    let log = entries();
    let mut writer = host(&log, true);
    publish(&writer, Body::Epoch { epoch: 1 });
    writer.set_epoch_key(1, [42; 32]);
    let relay = host(&writer.entries(), false);

    // The device's own host does not seal two contents under one stamp: the second would be sealed under
    // the same nonce as the first, and what the relay knows of one gives it the other.
    let first = sealed(&mut writer, 1000, "known-public").unwrap();
    let again = sealed(&mut writer, 1000, "secret-value");
    assert!(
        again.as_ref().is_err_and(|reason| reason.contains("stamp")),
        "{again:?}"
    );
    // What it made before still leaves as it was made.
    assert_eq!(
        sealed(&mut writer, 1000, "known-public").unwrap(),
        first,
        "one write is one set of bytes"
    );
    assert!(take(&relay, &first).unwrap().new);

    // A device that does not use this host signs two writes at two numbers under one stamp: they are both
    // kept, as proof, and the device is frozen from the first of them.
    let plain = host(&log, false);
    let (one, id_one) = forged(&plain, &key(1), 1, NO_OPERATION, 1000, 0, "a");
    let (two, _) = forged(&plain, &key(1), 2, id_one, 1000, 0, "b");
    assert!(take(&plain, &one).unwrap().applied);
    let taken = take(&plain, &two).unwrap();
    assert!(taken.new && !taken.applied, "kept as evidence, not applied");
    assert_eq!(plain.equivocating(), vec![(1, 1)]);
    assert!(held(&plain).is_empty());
    assert_eq!(plain.evidence().len(), 2);
}

#[test]
fn a_device_whose_own_host_has_found_its_chain_to_conflict_signs_nothing_more() {
    let log = entries();
    let mut writer = host(&log, true);
    sealed(&mut writer, 1000, "first").unwrap();
    // The same key signed another operation at number 1 somewhere else, and this host was given it.
    let elsewhere = host(&log, false);
    let (other, _) = forged(&elsewhere, &key(1), 1, NO_OPERATION, 1500, 0, "other");
    assert!(!take(&writer, &other).unwrap().applied);
    assert_eq!(writer.equivocating(), vec![(1, 1)]);

    let next = sealed(&mut writer, 3000, "next");
    assert!(
        next.as_ref()
            .is_err_and(|reason| reason.contains("conflicts at 1")),
        "{next:?}"
    );
}

#[test]
fn an_entry_nobody_signed_freezes_no_host() {
    let log = entries();
    let victim = host(&log, false);
    let mut writer = host(&log, true);

    // A hub rewrites an entry the victim holds, and takes the signatures off.
    let mut forged_entry = log[1].clone();
    forged_entry.body = Body::Epoch { epoch: 999 };
    forged_entry.signatures.clear();
    assert!(victim.append(&forged_entry).is_err());
    assert_eq!(victim.forked(), None, "nobody signed it: it proves nothing");

    // The victim goes on taking what is true.
    let good = sealed(&mut writer, 1000, "legitimate").unwrap();
    assert!(take(&victim, &good).unwrap().applied);
    for entry in &log {
        victim.append(entry).unwrap();
    }
    assert_eq!(victim.forked(), None);

    // An entry an administrator did sign, at a place where another is held, is a fork, and does freeze.
    let rival = Trust::invited(LIST, log[0].hash());
    rival.append(&log[0]).unwrap();
    let other = publish(
        &rival,
        Body::Grant {
            node: 2,
            role: 2,
            key: key(2).public(),
            final_seq: None,
        },
    );
    assert!(victim.append(&other).is_err());
    assert_eq!(victim.forked(), Some(1), "two histories, both signed");
}

#[test]
fn a_conflict_of_a_device_is_kept_as_evidence_that_travels_is_stored_and_is_audited() {
    let log = entries();
    let mut left = host(&log, true);
    let mut right = host(&log, true);
    let a = sealed(&mut left, 1000, "a").unwrap();
    let b = sealed(&mut right, 1000, "b").unwrap();

    let witness = host(&log, false);
    let peer = host(&log, false);
    take(&witness, &a).unwrap();
    take(&peer, &a).unwrap();
    let second = take(&witness, &b);
    assert!(
        second.as_ref().map_or(true, |taken| !taken.applied),
        "the second is not applied: {second:?}"
    );
    assert_eq!(witness.equivocating(), vec![(1, 1)]);
    // The machine was told the first, and is to forget it: the host says which.
    assert_eq!(witness.take_evicted(), vec![(1, 1)]);
    assert!(witness.take_restored().is_empty());

    // What the witness holds is given to a host that saw only one of the two: it learns of the fork.
    for (event, data) in witness.signed_operations() {
        let _ = peer.take_operation(&event, &data);
    }
    assert_eq!(peer.equivocating(), vec![(1, 1)]);
    assert!(held(&peer).is_empty(), "from there nothing is held");

    // The record an auditor is given says it, and checks out.
    let report = audit::verify(log[0].hash(), &audit::export(&witness)).unwrap();
    assert_eq!(report.equivocating, vec![(1, 1)]);
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    // And a host that restores what the witness stored holds it too.
    let restored = host(&log, false);
    restored.restore_from_text(&witness.to_text());
    assert_eq!(restored.equivocating(), vec![(1, 1)]);
}

#[test]
fn a_stamp_that_goes_back_gives_every_host_the_same_answer_in_either_order() {
    let log = entries();
    let plain = host(&log, false);
    let (late, id_late) = forged(&plain, &key(1), 1, NO_OPERATION, 2000, 0, "a");
    let (early, _) = forged(&plain, &key(1), 2, id_late, 1000, 0, "b");

    let left = host(&log, false);
    let right = host(&log, false);
    let _ = take(&left, &late);
    let _ = take(&left, &early);
    let _ = take(&right, &early);
    let _ = take(&right, &late);

    // The device signed a later write with an earlier time: both operations are proof of it, and
    // neither host holds either as its own.
    assert_eq!(left.equivocating(), vec![(1, 1)]);
    assert_eq!(right.equivocating(), vec![(1, 1)]);
    assert_eq!(held(&left), held(&right));
    assert!(held(&left).is_empty());
    assert_eq!(left.signed_operations(), right.signed_operations());
    assert_eq!(left.signed_operations().len(), 2);
}

#[test]
fn the_first_epoch_does_not_make_the_order_of_an_honest_plain_write_matter() {
    let log = entries();
    let writer_before = host(&log, false);
    let (plain, id_plain) = forged(&writer_before, &key(1), 1, NO_OPERATION, 1000, 0, "before");
    let epoch = publish(&host(&log, false), Body::Epoch { epoch: 1 });

    // The write was made before the list began to seal and names the entry before the epoch: a host
    // that takes it before the epoch holds it, and so does one that takes it after.
    let left = host(&log, false);
    take(&left, &plain).unwrap();
    left.append(&epoch).unwrap();
    let right = host(&log, false);
    right.append(&epoch).unwrap();
    assert!(take(&right, &plain).unwrap().applied);
    assert_eq!(held(&left), held(&right));
    assert_eq!(held(&left).len(), 1);

    // A write that names the entry that began the epoch, or one after, is sealed or it is refused.
    let (unsealed, _) = forged(&right, &key(1), 2, id_plain, 2000, 0, "after");
    assert!(
        matches!(take(&right, &unsealed), Err(TakeError::Refused(reason)) if reason.contains("not sealed"))
    );
}

#[test]
fn a_conflict_goes_when_the_log_puts_what_made_it_past_a_cutoff_and_what_it_held_back_is_told_again(
) {
    let log = entries();
    let plain = host(&log, false);
    let (late, id_late) = forged(&plain, &key(1), 1, NO_OPERATION, 2000, 0, "a");
    let (early, _) = forged(&plain, &key(1), 2, id_late, 1000, 0, "b");
    let removal = {
        let scratch = host(&log, false);
        publish(
            &scratch,
            Body::Grant {
                node: 1,
                role: 0,
                key: key(1).public(),
                final_seq: Some(1),
            },
        )
    };

    // One host takes both operations, and is then given the entry that takes the device off after the
    // first: the second is past the last accepted, the conflict goes with it, and the first is told
    // again to the machine, which had been told to forget it.
    let first = host(&log, false);
    take(&first, &late).unwrap();
    take(&first, &early).unwrap();
    assert_eq!(first.equivocating(), vec![(1, 1)]);
    assert!(held(&first).is_empty());
    first.take_evicted();
    first.append(&removal).unwrap();
    assert!(first.equivocating().is_empty(), "the conflict went");
    assert_eq!(first.take_restored(), vec![(1, 1)]);
    assert_eq!(held(&first).len(), 1);

    // Another that was given the entry before the operations holds the same.
    let second = host(&log, false);
    second.append(&removal).unwrap();
    take(&second, &late).unwrap();
    assert!(take(&second, &early).is_err(), "past what the log accepted");
    assert_eq!(held(&second), held(&first));
    assert!(second.equivocating().is_empty());
}

/// The log takes a device off after its first operation when the host has told its machine two: the
/// second is past what the log accepted, and the host tells the machine to forget it. No conflict is
/// involved, so this is the cutoff's own notice and not the freeze's: since a write that freezes a
/// chain evicts in `hold`, the cutoff is the only path that reaches `tell_what_changed`.
#[test]
fn what_a_cutoff_takes_off_a_device_the_machine_is_told_to_forget() {
    let log = entries();
    let plain = host(&log, false);
    let (first_op, id_first) = forged(&plain, &key(1), 1, NO_OPERATION, 1000, 0, "a");
    let (second_op, _) = forged(&plain, &key(1), 2, id_first, 2000, 0, "b");
    let removal = {
        let scratch = host(&log, false);
        publish(
            &scratch,
            Body::Grant {
                node: 1,
                role: 0,
                key: key(1).public(),
                final_seq: Some(1),
            },
        )
    };

    let one = host(&log, false);
    take(&one, &first_op).unwrap();
    take(&one, &second_op).unwrap();
    assert!(one.equivocating().is_empty(), "two operations in order");
    assert!(
        one.take_evicted().is_empty(),
        "nothing told was taken back yet"
    );
    one.append(&removal).unwrap();
    assert_eq!(
        one.take_evicted(),
        vec![(1, 2)],
        "the operation past the cutoff was told, and the machine is to forget it"
    );
    assert!(one.take_restored().is_empty());
}

#[test]
fn a_replacement_key_begins_the_chain_where_the_cutoff_ended_the_old_one() {
    let log = entries();
    let mut writer = host(&log, true);
    let first = sealed(&mut writer, 1000, "old key").unwrap();
    let peer = host(&log, false);
    take(&peer, &first).unwrap();

    // The administrator replaces node 1's key and ends the old chain at its first operation.
    let entry = publish(
        &writer,
        Body::Grant {
            node: 1,
            role: 2,
            key: key(2).public(),
            final_seq: Some(1),
        },
    );
    peer.append(&entry).unwrap();

    // The new key's first operation begins a new chain (number 2), naming nothing before it.
    writer.set_identity(1, key(2));
    let second = sealed(&mut writer, 2000, "new key").unwrap();
    assert!(
        take(&peer, &second).unwrap().applied,
        "the host that signed it and the host that takes it agree"
    );
    assert_eq!(held(&peer).len(), 2);
}
