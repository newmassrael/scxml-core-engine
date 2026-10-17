// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// Every place the trust crate reads what someone else wrote: the entries of the membership log,
// checkpoints and settlements, operations as they travel, the floor with its settlement, the text a
// host keeps its operations in, a grant of a key, a copy of a device's secrets, the record of an
// audit, and the encodings under them. Each is given a corpus of what it reads when it is true, and what
// a seeded mutator makes of the corpus (a bit, a cut, a hole, a repeat, a number that does not fit, a
// value of another type). A parser may refuse. It may not panic, and what it accepts it writes back as
// it read it.

mod common;

use common::fuzz;
use sce_rust_mesh::endpoint::{Admission, Sealer};
use sce_rust_trust::audit;
use sce_rust_trust::backup::{self, Contents, Cost};
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{link_bytes, Body, Entry, Log, Profile};
use sce_rust_trust::message::Link;
use sce_rust_trust::{Checkpoint, Identity, PublicKey, Settlement, Trust};
use serde_json::{json, Value};

const LIST: &str = "home";
const WEAK: Cost = Cost {
    memory_kib: 64,
    passes: 1,
    lanes: 1,
};

fn admin(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![admin(9).public(), admin(10).public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin(9))
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
    let entry = hosts[0].draft(body).sign(&admin(9)).finish();
    for host in hosts {
        host.append(&entry).unwrap();
    }
    entry
}

fn write(author: u64, id: u64, wall: i64) -> String {
    json!({"id": id.to_string(), "field": "3", "value": "0", "text": "",
           "wallTime": wall.to_string(), "counter": "0", "nodeId": author.to_string()})
    .to_string()
}

/// Everything the crate reads, as it is when it is true.
struct Corpus {
    entries: Vec<Entry>,
    checkpoints: Vec<Checkpoint>,
    settlements: Vec<Settlement>,
    operations: Vec<(String, String)>,
    floors: Vec<String>,
    store: String,
    grant: String,
    backup: String,
    audit: String,
    genesis_hash: [u8; 32],
}

fn corpus() -> Corpus {
    let hub = host(None);
    let (h1, h2, h3) = (host(Some(1)), host(Some(2)), host(Some(3)));
    let all = [&hub, &h1, &h2, &h3];
    let mut entries = vec![genesis()];
    for node in [1u64, 2, 3] {
        entries.push(publish(
            &all,
            Body::Grant {
                node,
                role: if node == 1 { 3 } else { 2 },
                key: person(node).public(),
                final_seq: None,
            },
        ));
    }
    let statement = link_bytes(LIST, 1, 5, &person(5).public());
    entries.push(publish(
        &all,
        Body::LinkedGrant {
            node: 5,
            role: 2,
            key: person(5).public(),
            parent: 1,
            parent_signature: person(1).sign(&statement),
            signature: person(5).sign(&statement),
        },
    ));
    entries.push(publish(&all, Body::Epoch { epoch: 1 }));
    for host in [&h1, &h2, &h3] {
        host.set_epoch_key(1, [5; 32]);
    }

    // Operations: bob writes three sealed writes; the hub is given the first and carol all three.
    let mut sealer = h2.clone();
    let mut operations: Vec<(String, String)> = (1..=3)
        .map(|n| {
            (
                "todo.write".to_string(),
                sealer
                    .seal("#s", "todo.write", &write(2, n, 1000 + n as i64))
                    .unwrap()
                    .unwrap(),
            )
        })
        .collect();
    let note =
        json!({"id": 1, "field": 3, "wallTime": "1001", "counter": 0, "nodeId": "2", "by": "1"})
            .to_string();
    operations.push((
        "todo.supersede".to_string(),
        h1.clone()
            .seal("#s", "todo.supersede", &note)
            .unwrap()
            .unwrap(),
    ));
    let (mut hub_host, mut h3_host) = (hub.clone(), h3.clone());
    hub_host
        .admit("r", &operations[0].0, &operations[0].1)
        .unwrap();
    for (event, data) in &operations[..3] {
        h3_host.admit("r", event, data).unwrap();
    }
    let before_removal = h3.checkpoint(30_000, 7).unwrap();
    let mut checkpoints = vec![before_removal.clone()];

    // A removal at the first operation, the amendment the checkpoint shows, an epoch and its
    // destruction, a change of the administrators and of the profile.
    entries.push(publish(
        &all,
        Body::Grant {
            node: 2,
            role: 0,
            key: person(2).public(),
            final_seq: Some(1),
        },
    ));
    entries.push(publish(
        &all,
        Body::Amend {
            node: 2,
            final_seq: 3,
            evidence: vec![before_removal],
        },
    ));
    entries.push(publish(&all, Body::Epoch { epoch: 2 }));
    entries.push(publish(&all, Body::Shred { epoch: 1 }));
    entries.push(publish(
        &all,
        Body::Administrators {
            add: vec![admin(11).public()],
            remove: vec![],
            quorum: 1,
        },
    ));
    entries.push(publish(&all, Body::Profile(Profile::TrustedHub)));
    checkpoints.push(h1.checkpoint(40_000, 8).unwrap());

    // An MLS list has its own kind of epoch entry.
    let mls = Trust::open("mls");
    let first = Log::genesis_draft(
        "mls",
        Body::Genesis {
            administrators: vec![admin(9).public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&admin(9))
    .finish();
    mls.append(&first).unwrap();
    entries.push(first);
    entries.push(publish(
        &[&mls],
        Body::MlsEpoch {
            epoch: 1,
            commit: [7; 32],
        },
    ));

    // A settlement and the floor that carries it, from a list of two writers.
    let (settle_hub, a, b) = (host(None), host(Some(1)), host(Some(2)));
    let pair = [&settle_hub, &a, &b];
    for node in [1u64, 2] {
        publish(
            &pair,
            Body::Grant {
                node,
                role: 2,
                key: person(node).public(),
                final_seq: None,
            },
        );
    }
    for member in [&a, &b] {
        settle_hub
            .take_checkpoint(&member.checkpoint(30_000, 7).unwrap())
            .unwrap();
    }
    let settlement = settle_hub.settlement(30_000, 7).unwrap();
    let floors = vec![
        json!({"before": "30000", "hash": 7, "cert": settlement.to_value()}).to_string(),
        json!({"before": "30000", "hash": 7}).to_string(),
    ];

    let grant = h1.grant_key(2, &person(2).public()).unwrap_or_else(|| {
        // Epoch 1 was destroyed and 2 was never given a key: make one that is given.
        h1.set_epoch_key(2, [6; 32]);
        h1.grant_key(2, &person(2).public()).unwrap()
    });
    let contents = Contents::new(LIST, 3, [3; 32])
        .with("mls", vec![1, 2, 3])
        .with_epoch_keys(&h3);
    Corpus {
        genesis_hash: genesis().hash(),
        entries,
        checkpoints,
        settlements: vec![settlement],
        operations,
        floors,
        store: hub.to_text(),
        grant,
        backup: backup::seal(b"pass", &contents, WEAK).unwrap(),
        audit: audit::export(&hub),
    }
}

fn bytes_of(texts: impl IntoIterator<Item = String>) -> Vec<Vec<u8>> {
    texts.into_iter().map(String::into_bytes).collect()
}

fn text(input: &[u8]) -> String {
    sce_rust_assurance::lossy(input)
}

#[test]
fn the_entries_of_the_membership_log_are_read_without_a_panic_and_written_back_as_read() {
    let corpus = corpus();
    let items = bytes_of(corpus.entries.iter().map(Entry::to_json));
    fuzz("entries", &items, 6000, |input| {
        if let Some(entry) = Entry::from_json(&text(input)) {
            assert_eq!(Entry::from_json(&entry.to_json()), Some(entry.clone()));
            let _ = entry.hash();
            let _ = entry.signing_bytes();
        }
    });
}

#[test]
fn checkpoints_and_settlements_are_read_without_a_panic_and_written_back_as_read() {
    let corpus = corpus();
    let items = bytes_of(corpus.checkpoints.iter().map(Checkpoint::to_json));
    fuzz("checkpoints", &items, 6000, |input| {
        if let Some(checkpoint) = Checkpoint::from_json(&text(input)) {
            assert_eq!(
                Checkpoint::from_json(&checkpoint.to_json()),
                Some(checkpoint.clone())
            );
            let _ = checkpoint.signing_bytes();
        }
    });
    let items = bytes_of(corpus.settlements.iter().map(|s| s.to_value().to_string()));
    fuzz("settlements", &items, 3000, |input| {
        if let Ok(value) = serde_json::from_slice::<Value>(input) {
            if let Some(settlement) = Settlement::from_value(&value) {
                assert_eq!(
                    Settlement::from_value(&settlement.to_value()),
                    Some(settlement)
                );
            }
        }
    });
}

#[test]
fn the_encodings_under_them_read_what_they_wrote_and_nothing_that_is_not() {
    let items: Vec<Vec<u8>> = [
        encoding::encode(&[0u8; 64]),
        encoding::encode(&[255u8; 32]),
        encoding::encode(b"a"),
        person(1).public().to_text(),
        "AAAA".to_string(),
        String::new(),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect();
    fuzz("encoding", &items, 6000, |input| {
        let input = text(input);
        if let Some(bytes) = encoding::decode(&input) {
            assert_eq!(encoding::encode(&bytes), input, "one text for one value");
        }
        if let Some(key) = PublicKey::from_text(&input) {
            assert_eq!(PublicKey::from_text(&key.to_text()), Some(key));
        }
    });
}

#[test]
fn what_a_host_is_given_as_an_operation_a_floor_a_store_or_a_grant_never_panics_it() {
    let corpus = corpus();

    // A host that has the log (and, to open content, bob's key) is given the operations mutated.
    let entries: Vec<Entry> = corpus.entries[..8].to_vec();
    let fresh = |own: Option<u64>| {
        let trust = Trust::invited(LIST, corpus.genesis_hash);
        for entry in entries.iter() {
            let _ = trust.append(entry);
        }
        if let Some(node) = own {
            trust.set_identity(node, person(node));
            trust.set_epoch_key(1, [5; 32]);
        }
        trust
    };
    for own in [None, Some(3)] {
        let items: Vec<Vec<u8>> = corpus
            .operations
            .iter()
            .map(|(_, data)| data.clone().into_bytes())
            .collect();
        fuzz("operations", &items, 2500, |input| {
            let trust = fresh(own);
            for event in ["todo.write", "todo.supersede"] {
                let _ = trust.take_operation(event, &text(input));
                let _ = trust.clone().admit("relay", event, &text(input));
            }
            let _ = Link::take_from(&mut serde_json::from_slice(input).unwrap_or(Value::Null));
        });
    }

    let items = bytes_of(corpus.floors.clone());
    fuzz("floors", &items, 2500, |input| {
        let mut trust = host(Some(1));
        let _ = trust.admit("hub", "todo.floor", &text(input));
        let mut allowed = host(None);
        allowed.allow_unsigned(&["todo.floor"]);
        let _ = allowed.admit("hub", "todo.floor", &text(input));
    });

    let items = bytes_of([corpus.store.clone()]);
    fuzz("store", &items, 1500, |input| {
        let trust = fresh(Some(3));
        let _ = trust.restore_from_text(&text(input));
        let _ = trust.to_text();
    });

    let items = bytes_of([corpus.grant.clone()]);
    fuzz("grant", &items, 2500, |input| {
        let _ = fresh(Some(2)).receive_grant(&text(input));
    });
}

#[test]
fn a_copy_of_a_devices_secrets_and_the_record_of_an_audit_are_read_without_a_panic() {
    let corpus = corpus();

    let items = bytes_of([corpus.backup.clone()]);
    fuzz("backup", &items, 400, |input| {
        // A copy that asks for a great deal of memory is refused before it is paid for
        // (`a_copy_that_asks_for_more_than_a_device_offers_is_refused_before_it_is_paid_for`); the
        // fuzz does not pay it either, so that it can run many inputs.
        let input = text(input);
        if let Ok(value) = serde_json::from_str::<Value>(&input) {
            let cost = |name: &str| value.get(name).and_then(Value::as_u64).unwrap_or(0);
            if cost("m")
                .saturating_mul(cost("t"))
                .saturating_mul(cost("p"))
                > 4096
            {
                return;
            }
        }
        if let Ok(contents) = backup::open(b"pass", &input) {
            let _ = contents.extra("mls");
        }
    });

    let items = bytes_of([corpus.audit.clone()]);
    fuzz("audit", &items, 500, |input| {
        let _ = audit::verify(corpus.genesis_hash, &text(input));
    });
}
