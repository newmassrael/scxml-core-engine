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
    let mut value = json!({"id": id.to_string(), "field": "3", "value": "1", "text": "",
        "wallTime": (1000 + seq as i64).to_string(), "counter": "0", "nodeId": node.to_string()});
    let signing = message::signing(LIST, "todo.write", &value)
        .unwrap()
        .unwrap();
    let (log, log_hash) = tip(host);
    let link = Link {
        seq,
        prev,
        log,
        log_hash,
    };
    let bytes = message::operation_bytes(&signing.message, &link);
    link.write_into(&mut value).unwrap();
    value[SIGNATURE_FIELD] = Value::String(encoding::encode(&signer.sign(&bytes)));
    (value.to_string(), message::operation_id(&bytes))
}

fn refused(host: &Trust, data: &str) -> String {
    match host.take_operation("todo.write", data) {
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
    assert!(host.take_operation("todo.write", &good).is_ok());
}

#[test]
fn a_new_key_does_not_reuse_the_numbers_of_the_chain_the_old_key_ended() {
    let host = host();
    grant(&host, 1, 2, 1);
    let (first, first_id) = operation(&host, &key(1), 1, 1, NO_OPERATION, 1);
    let (second, _) = operation(&host, &key(1), 1, 2, first_id, 2);
    host.take_operation("todo.write", &first).unwrap();
    host.take_operation("todo.write", &second).unwrap();
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
    assert!(host.take_operation("todo.write", &fresh).is_ok());
}

#[test]
fn a_chain_that_does_not_follow_itself_is_the_device_signing_two_histories() {
    // An operation that names another one before it than the host holds.
    let host_a = host();
    grant(&host_a, 1, 2, 1);
    let (first, first_id) = operation(&host_a, &key(1), 1, 1, NO_OPERATION, 1);
    host_a.take_operation("todo.write", &first).unwrap();
    let (wrong_prev, _) = operation(&host_a, &key(1), 1, 2, [7; 32], 2);
    let reason = refused(&host_a, &wrong_prev);
    assert!(
        reason.contains("signed two different operations at 1"),
        "{reason}"
    );
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
    host_b.take_operation("todo.write", &one).unwrap();
    host_b.take_operation("todo.write", &three).unwrap();
    let (other_two, _) = operation(&host_b, &key(1), 1, 2, one_id, 9);
    let reason = refused(&host_b, &other_two);
    assert!(
        reason.contains("signed two different operations at 2"),
        "{reason}"
    );
    assert_eq!(host_b.equivocating(), vec![(1, 2)]);
    // The true second, given now, is refused too: the device is frozen from 2.
    assert!(matches!(
        host_b.take_operation("todo.write", &two),
        Err(TakeError::Refused(_))
    ));
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
            "todo.floor",
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
        .admit("hub", "todo.floor", &floor_with(&settlement))
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
        .admit("hub", "todo.floor", &floor_with(&agreed))
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
