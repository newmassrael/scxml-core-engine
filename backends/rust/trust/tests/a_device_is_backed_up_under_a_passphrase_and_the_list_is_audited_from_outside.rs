// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// Two operational pieces. A copy of a device's secrets under a passphrase: it opens under the
// passphrase and no other, a changed copy does not open, one that asks a host for more than any
// device offers is refused before it is paid for, the keys of an epoch the list had destroyed are
// neither kept nor put back, and the bytes of a copy are pinned by a known-answer vector. And the
// record of a list that someone outside it checks with the hash of the genesis and no key: the log,
// the operations as they travelled and the checkpoints give a report of who was a member from which
// entry, which devices are one person's, how whole each device's chain is and what was destroyed; and a
// record that was changed fails where it was changed.

use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::audit::{self, AuditError};
use sce_rust_trust::backup::{self, BackupError, Contents, Cost};
use sce_rust_trust::membership::{link_bytes, Body, Entry, Log, LogError, Profile};
use sce_rust_trust::{Identity, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const WEAK: Cost = Cost {
    memory_kib: 64,
    passes: 1,
    lanes: 1,
};

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin())
    .finish()
}

fn host(own: Option<u64>) -> Trust {
    let trust = Trust::open(LIST);
    trust.append(&genesis()).unwrap();
    if let Some(node) = own {
        trust.set_identity(node, person(node));
    }
    trust
}

fn publish(hosts: &[&Trust], body: Body) -> Entry {
    let entry = hosts[0].draft(body).sign(&admin()).finish();
    for host in hosts {
        host.append(&entry).unwrap();
    }
    entry
}

fn write(author: u64, id: u64, wall: i64, text: &str) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "0", "text": text,
           "wallTime": wall.to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

#[test]
fn a_copy_of_a_devices_secrets_opens_under_its_passphrase_and_no_other() {
    let host = host(Some(3));
    publish(
        &[&host],
        Body::Grant {
            node: 3,
            role: 2,
            key: person(3).public(),
            final_seq: None,
        },
    );
    publish(&[&host], Body::Epoch { epoch: 1 });
    publish(&[&host], Body::Epoch { epoch: 2 });
    host.set_epoch_key(1, [11; 32]);
    host.set_epoch_key(2, [12; 32]);

    let contents = Contents::new(LIST, 3, [3; 32])
        .with("mls", vec![1, 2, 3, 4])
        .with_epoch_keys(&host);
    let text = backup::seal(b"a long passphrase", &contents, WEAK).unwrap();
    assert!(!text.contains("a long passphrase"));

    let opened = backup::open(b"a long passphrase", &text).unwrap();
    assert_eq!(opened, contents);
    assert_eq!(
        Identity::from_seed(opened.seed).public(),
        person(3).public(),
        "the seed makes the device's key again"
    );
    assert_eq!(opened.extra("mls"), Some([1u8, 2, 3, 4].as_slice()));
    assert_eq!(
        backup::open(b"another passphrase", &text),
        Err(BackupError::Corrupt)
    );

    // Put back into a host of the same list, the keys are what they were.
    let replacement = Trust::open(LIST);
    for entry in host.entries() {
        replacement.append(&entry).unwrap();
    }
    assert_eq!(opened.restore_epoch_keys(&replacement), 2);
    assert_eq!(replacement.epochs(), vec![1, 2]);
}

#[test]
fn the_keys_of_an_epoch_the_list_destroyed_are_neither_kept_nor_put_back() {
    let host = host(Some(3));
    publish(
        &[&host],
        Body::Grant {
            node: 3,
            role: 2,
            key: person(3).public(),
            final_seq: None,
        },
    );
    publish(&[&host], Body::Epoch { epoch: 1 });
    publish(&[&host], Body::Epoch { epoch: 2 });
    host.set_epoch_key(1, [11; 32]);
    host.set_epoch_key(2, [12; 32]);

    // A copy made before the list destroyed epoch 1 holds its key.
    let before = Contents::new(LIST, 3, [3; 32]).with_epoch_keys(&host);
    assert!(before.extra("epoch-key/1").is_some());

    publish(&[&host], Body::Shred { epoch: 1 });
    let after = Contents::new(LIST, 3, [3; 32]).with_epoch_keys(&host);
    assert!(
        after.extra("epoch-key/1").is_none(),
        "a copy made now keeps none"
    );
    assert!(after.extra("epoch-key/2").is_some());

    // The old copy, put back into a host that has the log, gives the key of epoch 1 nowhere.
    assert_eq!(before.restore_epoch_keys(&host), 1, "only epoch 2");
    assert_eq!(host.epochs(), vec![2]);
}

#[test]
fn a_copy_that_was_changed_or_asks_too_much_is_refused() {
    let contents = Contents::new(LIST, 3, [3; 32]);
    let text = backup::seal(b"pass", &contents, WEAK).unwrap();
    let mut value: Value = serde_json::from_str(&text).unwrap();
    // The first character becomes another one whatever it was: a copy sealed with a random nonce can
    // begin with the character put there, and then nothing would have been changed.
    let ciphertext = value["ct"].as_str().unwrap().to_string();
    let other = if ciphertext.starts_with('A') {
        'B'
    } else {
        'A'
    };
    value["ct"] = json!(format!("{other}{}", &ciphertext[1..]));
    assert_ne!(
        value["ct"].as_str().unwrap(),
        ciphertext,
        "the copy was changed"
    );
    assert_eq!(
        backup::open(b"pass", &value.to_string()),
        Err(BackupError::Corrupt)
    );
    let mut greedy: Value = serde_json::from_str(&text).unwrap();
    greedy["m"] = json!(8_000_000);
    assert_eq!(
        backup::open(b"pass", &greedy.to_string()),
        Err(BackupError::TooCostly)
    );
    assert_eq!(
        backup::open(b"pass", "not a copy"),
        Err(BackupError::Malformed)
    );
}

/// The bytes of a copy are a contract with every device that opens one, for as long as the copy
/// lives: a change to the cost, the header, the key derivation or the layout of the contents makes the
/// copies that exist stop opening, and shows here.
#[test]
fn the_bytes_of_a_copy_are_written_as_they_always_were() {
    let contents = Contents::new(LIST, 4, [7; 32]).with("mls", vec![1, 2, 3]);
    let text = backup::seal_with(b"correct horse", &contents, WEAK, [1; 16], [2; 24]).unwrap();
    assert_eq!(
        text,
        r#"{"ct":"YFSIr5lWYTiJBi-DTO1mIL65LM-7P14WpRrBB9xRr3GpFVkqJHxHtLAih_ePN1r0ZgmXDWmPePwpI2OCO5nG_e2MIIghy0iqUO9F5sOGFURbH7K2qoVJiRvyFYR5-YNid8AP603WkSdXeHQx4AbdfUq5dpOyVf3jdEYvOQbZ2w","kdf":"argon2id","m":64,"nonce":"AgICAgICAgICAgICAgICAgICAgICAgIC","p":1,"salt":"AQEBAQEBAQEBAQEBAQEBAQ","t":1,"v":1}"#
    );
    assert_eq!(backup::open(b"correct horse", &text), Ok(contents));
}

/// A list with a linked device, an epoch, writes by two devices, a removal and a checkpoint, and the
/// hub's host that holds all of it.
fn recorded() -> (Trust, Entry, Vec<String>) {
    let hub = host(None);
    let alice = host(Some(1));
    let bob = host(Some(2));
    let hosts = [&hub, &alice, &bob];
    publish(
        &hosts,
        Body::Grant {
            node: 1,
            role: 3,
            key: person(1).public(),
            final_seq: None,
        },
    );
    publish(
        &hosts,
        Body::Grant {
            node: 2,
            role: 2,
            key: person(2).public(),
            final_seq: None,
        },
    );
    let statement = link_bytes(LIST, 1, 5, &person(5).public());
    publish(
        &hosts,
        Body::LinkedGrant {
            node: 5,
            role: 2,
            key: person(5).public(),
            parent: 1,
            parent_signature: person(1).sign(&statement),
            signature: person(5).sign(&statement),
        },
    );
    publish(&hosts, Body::Epoch { epoch: 1 });
    alice.set_epoch_key(1, [5; 32]);
    bob.set_epoch_key(1, [5; 32]);

    let mut ops = Vec::new();
    let (mut alice_host, mut bob_host) = (alice.clone(), bob.clone());
    for (n, text) in ["old secret one", "old secret two", "old secret three"]
        .into_iter()
        .enumerate()
    {
        ops.push(
            alice_host
                .seal(
                    "#s",
                    "todo.write",
                    &write(1, n as u64 + 1, 1000 + n as i64, text),
                )
                .unwrap()
                .unwrap(),
        );
    }
    for (n, text) in ["bob one", "bob two"].into_iter().enumerate() {
        ops.push(
            bob_host
                .seal(
                    "#s",
                    "todo.write",
                    &write(2, 10 + n as u64, 2000 + n as i64, text),
                )
                .unwrap()
                .unwrap(),
        );
    }
    let mut hub_host = hub.clone();
    for op in &ops {
        hub_host.admit("relay", "todo.write", op).unwrap();
    }
    let bobs = bob.checkpoint(30_000, 7).unwrap();
    alice.take_checkpoint(&bobs).unwrap();
    hub.take_checkpoint(&bobs).unwrap();
    hub.take_checkpoint(&alice.checkpoint(30_000, 7).unwrap())
        .unwrap();
    publish(
        &hosts,
        Body::Grant {
            node: 2,
            role: 0,
            key: person(2).public(),
            final_seq: Some(hub.last_seq(2)),
        },
    );
    let genesis = hub.entries()[0].clone();
    (hub, genesis, ops)
}

#[test]
fn a_list_is_audited_with_the_hash_of_its_genesis_and_no_key() {
    let (hub, genesis, _) = recorded();
    let text = audit::export(&hub);
    for secret in ["old secret", "bob one", "bob two"] {
        assert!(!text.contains(secret), "the record holds {secret}");
    }

    let report = audit::verify(genesis.hash(), &text).unwrap();
    assert_eq!(report.list, LIST);
    assert_eq!(report.entries, 6);
    assert_eq!(report.administrators, vec![admin().public()]);
    assert_eq!(
        (report.epoch, report.mls, report.profile),
        (1, false, Profile::Untrusted)
    );
    assert!(report.shredded.is_empty());
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert!(report.equivocating.is_empty() && report.divergences.is_empty());
    assert_eq!(report.checkpoints, 2);

    let member = |node: u64| report.members.iter().find(|m| m.node == node).unwrap();
    assert_eq!(member(5).person, 1, "the phone is alice's");
    assert_eq!(member(1).person, 1);
    assert_eq!(member(2).person, 2);
    let bob = member(2);
    assert_eq!(bob.eras.len(), 2);
    assert_eq!((bob.eras[0].role, bob.eras[0].cutoff), (2, None));
    assert_eq!(
        (bob.eras[1].role, bob.eras[1].cutoff),
        (0, Some(2)),
        "taken off at his second"
    );

    let chain = |node: u64| report.chains.iter().find(|c| c.node == node).unwrap();
    assert_eq!(
        (chain(1).operations, chain(1).whole_to, chain(1).last),
        (3, 3, 3)
    );
    assert_eq!(
        (chain(2).operations, chain(2).whole_to, chain(2).last),
        (2, 2, 2)
    );
}

#[test]
fn an_audit_names_the_epochs_the_list_destroyed() {
    let hub = host(None);
    let genesis = genesis();
    publish(&[&hub], Body::Epoch { epoch: 1 });
    publish(&[&hub], Body::Epoch { epoch: 2 });
    publish(&[&hub], Body::Shred { epoch: 1 });

    let report = audit::verify(genesis.hash(), &audit::export(&hub)).unwrap();
    assert_eq!(report.shredded, vec![1]);
    assert_eq!(report.epoch, 2);
}

#[test]
fn a_record_that_was_changed_fails_where_it_was_changed() {
    let (hub, genesis, _) = recorded();
    let text = audit::export(&hub);
    let hash = genesis.hash();

    // Not the genesis the auditor was told.
    assert_eq!(
        audit::verify([0; 32], &text),
        Err(AuditError::NotTheGenesis)
    );

    // An entry whose role was raised: the signature no longer holds, and the log stops there.
    let mut value: Value = serde_json::from_str(&text).unwrap();
    let mut entry: Value = serde_json::from_str(value["entries"][2].as_str().unwrap()).unwrap();
    entry["body"]["role"] = json!(3);
    value["entries"][2] = Value::String(entry.to_string());
    assert!(matches!(
        audit::verify(hash, &value.to_string()),
        Err(AuditError::Log {
            index: 2,
            error: LogError::BadSignature
        })
    ));

    // An operation whose number was changed is listed as not holding, and counts for nothing.
    let mut value: Value = serde_json::from_str(&text).unwrap();
    let mut op: Value = serde_json::from_str(value["operations"][0][1].as_str().unwrap()).unwrap();
    op["value"] = json!("9");
    value["operations"][0][1] = Value::String(op.to_string());
    let report = audit::verify(hash, &value.to_string()).unwrap();
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    let chain = |node: u64| report.chains.iter().find(|c| c.node == node).unwrap();
    assert_eq!(chain(1).operations, 2);
    assert_eq!(
        chain(1).whole_to,
        0,
        "alice's chain has lost its first, and says so"
    );

    // An operation taken out of the middle of a chain leaves a gap that the report shows.
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["operations"].as_array_mut().unwrap().remove(1);
    let report = audit::verify(hash, &value.to_string()).unwrap();
    let alice = report.chains.iter().find(|c| c.node == 1).unwrap();
    assert_eq!((alice.operations, alice.whole_to, alice.last), (2, 1, 3));

    // A log cut short leaves operations that name entries the record does not hold.
    let mut value: Value = serde_json::from_str(&text).unwrap();
    value["entries"].as_array_mut().unwrap().truncate(3);
    let report = audit::verify(hash, &value.to_string()).unwrap();
    assert!(!report.refused.is_empty());
    assert_eq!(report.entries, 3);

    // Not a record.
    assert!(matches!(
        audit::verify(hash, "not json"),
        Err(AuditError::Malformed(_))
    ));
    assert!(matches!(
        audit::verify(
            hash,
            r#"{"v":1,"list":"home","entries":[],"operations":[],"checkpoints":[]}"#
        ),
        Err(AuditError::Malformed("the log has no entry"))
    ));
}
