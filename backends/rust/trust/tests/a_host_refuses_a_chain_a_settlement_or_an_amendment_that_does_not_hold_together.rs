// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// What a host refuses when the signer is a device that holds a key and means harm, so that the
// refusal cannot be the work of an honest host's own bookkeeping. An honest host does not make an
// operation that names the wrong thing before it, a checkpoint that claims another host's chain, or a
// checkpoint under another entry than its peers; these are made here with the key, by hand, and the host
// that receives them has to be the one that says no. They are the checks a mutation round over the
// verification code found nothing else was holding in place: the start of a chain, the numbers an earlier
// key's chain used, a chain that does not follow itself, a settlement of checkpoints under different
// entries or naming different operations, and an amendment shown by the device that was taken off or by a
// checkpoint made after the removal.

use sce_rust_mesh::endpoint::Admission;
use sce_rust_trust::checkpoint::{Checkpoint, Head, Settlement};
use sce_rust_trust::membership::{Body, Entry, Log, LogError, Profile};
use sce_rust_trust::message::{self, Link, NO_OPERATION, SIGNATURE_FIELD};
use sce_rust_trust::{encoding, Identity, TakeError, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn key(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn host() -> Trust {
    let trust = Trust::open(LIST);
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
    trust.append(&genesis).unwrap();
    trust
}

fn publish(host: &Trust, body: Body) -> Entry {
    let entry = host.draft(body).sign(&admin()).finish();
    host.append(&entry).unwrap();
    entry
}

fn grant(host: &Trust, node: u64, role: u8, seed: u8) -> Entry {
    publish(
        host,
        Body::Grant {
            node,
            role,
            key: key(seed).public(),
            final_seq: None,
        },
    )
}

/// The entry the host is at now: where an operation made under it stands, and its hash.
fn tip(host: &Trust) -> (u64, [u8; 32]) {
    let (next, hash) = host.head();
    (next - 1, hash)
}

/// An operation of `node`, signed by `signer` as the device makes it, naming `prev` and the number
/// `seq`: made by hand, so that what it says of its chain is what the test wants and not what an
/// honest host would write.
fn operation(
    host: &Trust,
    signer: &Identity,
    node: u64,
    seq: u64,
    prev: [u8; 32],
    id: u64,
) -> (String, [u8; 32]) {
    stamped(host, signer, node, seq, prev, id, 1000 + seq as i64, "1")
}

/// [`operation`], with the write's time and value given: what makes a time that goes back along a
/// chain, or two contents under one stamp.
#[allow(clippy::too_many_arguments)]
fn stamped(
    host: &Trust,
    signer: &Identity,
    node: u64,
    seq: u64,
    prev: [u8; 32],
    id: u64,
    wall: i64,
    content: &str,
) -> (String, [u8; 32]) {
    let mut value = json!({"id": id.to_string(), "field": "3", "value": content, "text": "",
        "wallTime": wall.to_string(), "counter": "0", "nodeId": node.to_string()});
    let signing = message::signing(LIST, "record_list.write", &value)
        .unwrap()
        .unwrap();
    let (log, log_hash) = tip(host);
    let link = Link {
        seq,
        prev,
        log,
        log_hash,
        context: Vec::new(),
    };
    let bytes = message::operation_bytes(&signing.message, &link);
    link.write_into(&mut value).unwrap();
    value[SIGNATURE_FIELD] = Value::String(encoding::encode(&signer.sign(&bytes)));
    (value.to_string(), message::operation_id(&bytes))
}

fn refused(host: &Trust, data: &str) -> String {
    match host.take_operation("record_list.write", data) {
        Err(TakeError::Refused(reason)) => reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_chain_begins_at_one_and_the_first_operation_names_nothing_before_it() {
    let host = host();
    grant(&host, 1, 2, 1);
    let (zero, _) = operation(&host, &key(1), 1, 0, NO_OPERATION, 1);
    assert!(refused(&host, &zero).contains("begins at 1"));
    let (first, _) = operation(&host, &key(1), 1, 1, [9; 32], 2);
    assert!(refused(&host, &first).contains("names an operation before it"));
    let (good, _) = operation(&host, &key(1), 1, 1, NO_OPERATION, 3);
    assert!(host.take_operation("record_list.write", &good).is_ok());
}

#[test]
fn a_new_key_does_not_reuse_the_numbers_of_the_chain_the_old_key_ended() {
    let host = host();
    grant(&host, 1, 2, 1);
    let (first, first_id) = operation(&host, &key(1), 1, 1, NO_OPERATION, 1);
    let (second, _) = operation(&host, &key(1), 1, 2, first_id, 2);
    host.take_operation("record_list.write", &first).unwrap();
    host.take_operation("record_list.write", &second).unwrap();
    // The device's key is replaced, and its chain ended at 2.
    publish(
        &host,
        Body::Grant {
            node: 1,
            role: 2,
            key: key(21).public(),
            final_seq: Some(2),
        },
    );
    // The new key's operation numbered 2 would be the old chain's: it belongs to the chain that ended.
    let (inside, _) = operation(&host, &key(21), 1, 2, NO_OPERATION, 5);
    let reason = refused(&host, &inside);
    assert!(
        reason.contains("belongs to the chain an earlier grant ended at 2"),
        "{reason}"
    );
    // Numbered past it, it begins the new chain, which names nothing before it.
    let (fresh, _) = operation(&host, &key(21), 1, 3, NO_OPERATION, 6);
    assert!(host.take_operation("record_list.write", &fresh).is_ok());
}

#[test]
fn a_chain_that_does_not_follow_itself_is_the_device_signing_two_histories() {
    // An operation that names another one before it than the host holds.
    let host_a = host();
    grant(&host_a, 1, 2, 1);
    let (first, first_id) = operation(&host_a, &key(1), 1, 1, NO_OPERATION, 1);
    host_a.take_operation("record_list.write", &first).unwrap();
    let (wrong_prev, _) = operation(&host_a, &key(1), 1, 2, [7; 32], 2);
    assert!(held_as_proof(&host_a, &wrong_prev));
    assert_eq!(host_a.equivocating(), vec![(1, 1)]);
    let _ = first_id;

    // An operation that the one after it, which the host holds, does not name.
    let host_b = host();
    grant(&host_b, 1, 2, 1);
    let (one, one_id) = operation(&host_b, &key(1), 1, 1, NO_OPERATION, 1);
    let (two, two_id) = operation(&host_b, &key(1), 1, 2, one_id, 2);
    let (three, _) = operation(&host_b, &key(1), 1, 3, two_id, 3);
    // The host holds the first and the third (which names the true second), and then is given another
    // second.
    host_b.take_operation("record_list.write", &one).unwrap();
    host_b.take_operation("record_list.write", &three).unwrap();
    let (other_two, _) = operation(&host_b, &key(1), 1, 2, one_id, 9);
    assert!(held_as_proof(&host_b, &other_two));
    assert_eq!(host_b.equivocating(), vec![(1, 2)]);
    // The true second, given now, is held as proof too, and nothing is told of the device from 2: it
    // conflicts with the other second, and the one the host held before.
    assert!(held_as_proof(&host_b, &two));
    assert_eq!(host_b.equivocating(), vec![(1, 2)]);
    assert_eq!(host_b.all_operations().len(), 1, "only the first is told");
    assert_eq!(host_b.evidence().len(), 3);
}

/// An operation that holds and conflicts with what the host holds of its device is taken in as proof:
/// it is held, and the machine is told nothing of the device from the first conflict on.
fn held_as_proof(host: &Trust, data: &str) -> bool {
    let taken = host
        .take_operation("record_list.write", data)
        .expect("held as proof, not refused");
    !taken.applied
}

/// A checkpoint of `device` under the entry at `log`, naming `heads`, signed by `signer`: made by
/// hand, as a device that holds its key and means to say what it did not hold.
fn checkpoint(
    host: &Trust,
    signer: &Identity,
    device: u64,
    log: u64,
    heads: Vec<Head>,
) -> Checkpoint {
    let mut checkpoint = Checkpoint {
        list: LIST.to_string(),
        device,
        log,
        log_hash: host.entries()[log as usize].hash(),
        before: 30_000,
        digest: 7,
        heads,
        signature: [0; 64],
    };
    checkpoint.signature = signer.sign(&checkpoint.signing_bytes());
    checkpoint
}

fn floor_with(settlement: &Settlement) -> String {
    json!({"before": "30000", "hash": 7, "cert": settlement.to_value()}).to_string()
}

#[test]
fn a_settlement_of_checkpoints_under_different_entries_is_not_one() {
    let host = host();
    grant(&host, 1, 2, 1);
    grant(&host, 2, 2, 2);
    let alice = checkpoint(&host, &key(1), 1, 2, vec![]);
    // The administrator adds a reader; bob speaks under the entry after.
    grant(&host, 3, 1, 3);
    let bob = checkpoint(&host, &key(2), 2, 3, vec![]);
    let mut receiver = host.clone();
    let reason = receiver
        .admit(
            "hub",
            "record_list.floor",
            &floor_with(&Settlement {
                checkpoints: vec![alice, bob],
            }),
        )
        .unwrap_err();
    assert!(reason.contains("different entries of the log"), "{reason}");
}

#[test]
fn a_settlement_whose_checkpoints_name_different_operations_at_one_number_is_not_one() {
    let host = host();
    for node in [1u64, 2, 3] {
        grant(&host, node, 2, node as u8);
    }
    let level = tip(&host).0;
    let head = |id: u8| {
        vec![Head {
            node: 3,
            seq: 1,
            id: [id; 32],
        }]
    };
    let settlement = Settlement {
        checkpoints: vec![
            checkpoint(&host, &key(1), 1, level, head(1)),
            checkpoint(&host, &key(2), 2, level, head(2)),
            checkpoint(&host, &key(3), 3, level, vec![]),
        ],
    };
    let mut receiver = host.clone();
    let reason = receiver
        .admit("hub", "record_list.floor", &floor_with(&settlement))
        .unwrap_err();
    assert!(
        reason.contains("different operations at one number"),
        "{reason}"
    );
    // The same three, naming the same operation, are a settlement.
    let agreed = Settlement {
        checkpoints: vec![
            checkpoint(&host, &key(1), 1, level, head(1)),
            checkpoint(&host, &key(2), 2, level, head(1)),
            checkpoint(&host, &key(3), 3, level, vec![]),
        ],
    };
    assert!(receiver
        .admit("hub", "record_list.floor", &floor_with(&agreed))
        .is_ok());
}

/// An amendment is shown by what other writers held before the removal. Here the evidence is made by
/// hand by a device that holds its key, so that nothing but the log's own checks can refuse it.
#[test]
fn an_amendment_is_not_shown_by_the_device_taken_off_or_by_a_checkpoint_made_after_it() {
    let host = host();
    for node in [1u64, 2, 3] {
        grant(&host, node, 2, node as u8);
    }
    let before_removal = tip(&host).0;
    let reaches = |node: u64| {
        vec![Head {
            node,
            seq: 3,
            id: [5; 32],
        }]
    };
    // Bob's own word, made while he was a writer, that his chain was whole to 3.
    let own = checkpoint(&host, &key(2), 2, before_removal, reaches(2));
    // Carol's word, honest and made before the removal.
    let carols = checkpoint(&host, &key(3), 3, before_removal, reaches(2));
    publish(
        &host,
        Body::Grant {
            node: 2,
            role: 0,
            key: key(2).public(),
            final_seq: Some(1),
        },
    );
    // Carol's word, made under the removal entry itself and claiming bob's chain to 3: an honest
    // host would have dropped what the removal put out, so only a device that means to lie says it.
    let after = checkpoint(&host, &key(3), 3, tip(&host).0, reaches(2));

    let amend = |evidence: Vec<Checkpoint>| {
        let entry = host
            .draft(Body::Amend {
                node: 2,
                final_seq: 3,
                evidence,
            })
            .sign(&admin())
            .finish();
        host.append(&entry)
    };
    let shown = LogError::Invalid(
        "an amendment is shown by a checkpoint of another writer, made before the removal",
    );
    assert_eq!(
        amend(vec![own]),
        Err(shown.clone()),
        "the device taken off vouches for itself"
    );
    assert_eq!(
        amend(vec![after]),
        Err(shown),
        "made under the removal entry"
    );
    assert_eq!(amend(vec![carols]), Ok(true));
}

/// What an operation taken in says of how it conflicts with what the host held of its device
/// (`Taken::conflict`), for each way two operations conflict: the position, where the chain was
/// frozen before and is now, whether it is the one that froze it, and the operations held it
/// conflicts with, as they travel. One held already, one that follows the chain, and one past a gap
/// conflict with nothing. A relay answers by it (ADR 0083 section 4).
#[test]
fn an_operation_that_conflicts_says_where_with_what_and_whether_it_froze_its_device() {
    const WRITE: &str = "record_list.write";
    let parsed = |data: &str| serde_json::from_str::<Value>(data).unwrap();
    let with = |taken: &sce_rust_trust::Taken| -> Vec<Value> {
        let conflict = taken.conflict.as_ref().expect("it conflicts");
        assert!(conflict.with.iter().all(|(event, _)| event == WRITE));
        conflict.with.iter().map(|(_, data)| parsed(data)).collect()
    };

    let host_a = host();
    grant(&host_a, 1, 2, 1);
    let (one, one_id) = operation(&host_a, &key(1), 1, 1, NO_OPERATION, 1);
    let (two, two_id) = operation(&host_a, &key(1), 1, 2, one_id, 2);
    // Past a gap: it names an operation at 3 the host is never given.
    let (four, _) = operation(&host_a, &key(1), 1, 4, [5; 32], 4);
    for data in [&one, &two, &four] {
        let taken = host_a.take_operation(WRITE, data).unwrap();
        assert!(taken.new && taken.applied, "{taken:?}");
        assert_eq!(taken.conflict, None);
    }
    let again = host_a.take_operation(WRITE, &two).unwrap();
    assert!(!again.new);
    assert_eq!(
        again.conflict, None,
        "one held already conflicts with nothing"
    );

    // Another second, at the number of the true one: it froze the device at 2.
    let (other_two, _) = operation(&host_a, &key(1), 1, 2, one_id, 9);
    let taken = host_a.take_operation(WRITE, &other_two).unwrap();
    assert!(taken.new && !taken.applied);
    let conflict = taken.conflict.clone().unwrap();
    assert_eq!(
        (
            conflict.at,
            conflict.before,
            conflict.after,
            conflict.froze()
        ),
        (2, None, 2, true)
    );
    assert_eq!(with(&taken), vec![parsed(&two)]);
    assert_eq!(host_a.equivocating(), vec![(1, 2)]);

    // A third second: it conflicts with both held there, and the chain was frozen there already.
    let (third_two, _) = operation(&host_a, &key(1), 1, 2, one_id, 10);
    let taken = host_a.take_operation(WRITE, &third_two).unwrap();
    let conflict = taken.conflict.clone().unwrap();
    assert_eq!(
        (
            conflict.at,
            conflict.before,
            conflict.after,
            conflict.froze()
        ),
        (2, Some(2), 2, false)
    );
    let mut found = with(&taken);
    found.sort_by_key(Value::to_string);
    let mut expected = vec![parsed(&two), parsed(&other_two)];
    expected.sort_by_key(Value::to_string);
    assert_eq!(found, expected);

    // A third that follows the true second, and that the fourth held does not name: it conflicts
    // at 3 with the fourth, above where the chain is frozen.
    let (three, _) = operation(&host_a, &key(1), 1, 3, two_id, 3);
    let taken = host_a.take_operation(WRITE, &three).unwrap();
    let conflict = taken.conflict.clone().unwrap();
    assert_eq!(
        (
            conflict.at,
            conflict.before,
            conflict.after,
            conflict.froze()
        ),
        (3, Some(2), 2, false)
    );
    assert_eq!(with(&taken), vec![parsed(&four)]);

    // One that names another before it than the host holds there: it conflicts with the one held.
    let host_b = host();
    grant(&host_b, 1, 2, 1);
    let (first, _) = operation(&host_b, &key(1), 1, 1, NO_OPERATION, 1);
    host_b.take_operation(WRITE, &first).unwrap();
    let (wrong_prev, _) = operation(&host_b, &key(1), 1, 2, [7; 32], 2);
    let taken = host_b.take_operation(WRITE, &wrong_prev).unwrap();
    let conflict = taken.conflict.clone().unwrap();
    assert_eq!((conflict.at, conflict.before, conflict.after), (1, None, 1));
    assert_eq!(with(&taken), vec![parsed(&first)]);

    // A write whose time goes back along the chain, given after the later one and given before it.
    for later_first in [true, false] {
        let host_c = host();
        grant(&host_c, 1, 2, 1);
        let (late, late_id) = stamped(&host_c, &key(1), 1, 1, NO_OPERATION, 1, 2000, "1");
        let (back, _) = stamped(&host_c, &key(1), 1, 2, late_id, 2, 1500, "1");
        let (held, given) = if later_first {
            (&late, &back)
        } else {
            (&back, &late)
        };
        assert_eq!(host_c.take_operation(WRITE, held).unwrap().conflict, None);
        let taken = host_c.take_operation(WRITE, given).unwrap();
        let conflict = taken.conflict.clone().unwrap();
        assert_eq!(
            (conflict.at, conflict.after),
            (1, 1),
            "later first: {later_first}"
        );
        assert_eq!(
            with(&taken),
            vec![parsed(held)],
            "later first: {later_first}"
        );
    }

    // Two contents under one stamp: the second conflicts with the first, at the lower number.
    let host_d = host();
    grant(&host_d, 1, 2, 1);
    let (once, once_id) = stamped(&host_d, &key(1), 1, 1, NO_OPERATION, 1, 1001, "1");
    let (twice, _) = stamped(&host_d, &key(1), 1, 2, once_id, 1, 1001, "2");
    host_d.take_operation(WRITE, &once).unwrap();
    let taken = host_d.take_operation(WRITE, &twice).unwrap();
    let conflict = taken.conflict.clone().unwrap();
    assert_eq!((conflict.at, conflict.after), (1, 1));
    assert_eq!(with(&taken), vec![parsed(&once)]);
}
