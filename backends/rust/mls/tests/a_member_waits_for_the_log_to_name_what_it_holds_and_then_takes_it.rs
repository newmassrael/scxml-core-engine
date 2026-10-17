// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A host is given the group's messages by a relay in whatever order the relay has them, and the
// entries of the log that name them by whatever other way. What is asked here: that a Welcome or a
// commit that the log has not named yet is kept and not refused, so that it is used once the entry
// that names it arrives, and that nothing is done with it before.

use sce_rust_mls::host::{Member, Step};
use sce_rust_mls::Device;
use sce_rust_relay::Record;
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};

const LIST: &str = "home";

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn identity(node: u64) -> Identity {
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

/// The administrator writes `body` into the log `trust` holds, and gives the entry back.
fn publish(trust: &Trust, body: Body) -> Entry {
    let entry = trust.draft(body).sign(&admin()).finish();
    trust.append(&entry).unwrap();
    entry
}

fn grant(trust: &Trust, node: u64, role: u8) -> Entry {
    publish(
        trust,
        Body::Grant {
            node,
            role,
            key: identity(node).public(),
            final_seq: None,
        },
    )
}

fn host(node: u64, entries: &[Entry]) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries {
        trust.append(entry).unwrap();
    }
    trust.set_identity(node, identity(node));
    trust
}

#[test]
fn a_welcome_and_a_commit_the_log_has_not_named_yet_wait_for_the_entry_and_are_then_taken() {
    // The administrator's host: alice runs the group, and adds bob and then carol.
    let alice_trust = host(1, &[genesis()]);
    let alice_grant = grant(&alice_trust, 1, 3);
    let mut alice = Device::new(LIST, 1, identity(1));
    alice.create_group().unwrap();
    alice.verify_roster(&alice_trust).unwrap();

    let mut bob = Device::new(LIST, 2, identity(2));
    let mut carol = Device::new(LIST, 3, identity(3));
    let (bob_package, carol_package) = (bob.key_package().unwrap(), carol.key_package().unwrap());

    let bob_grant = grant(&alice_trust, 2, 2);
    let first = alice.stage_add(&[bob_package], &alice_trust).unwrap();
    let first_entry = publish(
        &alice_trust,
        Body::MlsEpoch {
            epoch: 1,
            commit: first.commit_hash,
        },
    );
    alice.merge_pending(&alice_trust).unwrap();

    let carol_grant = grant(&alice_trust, 3, 2);
    let second = alice.stage_add(&[carol_package], &alice_trust).unwrap();
    let second_entry = publish(
        &alice_trust,
        Body::MlsEpoch {
            epoch: 2,
            commit: second.commit_hash,
        },
    );
    alice.merge_pending(&alice_trust).unwrap();

    // Bob's host has the log as far as his own grant, and not yet the entry that names the commit of
    // epoch 1, nor what follows it.
    let mut bob_member = Member::new(bob, host(2, &[genesis(), alice_grant, bob_grant]), 2);
    bob_member.absorb(&[
        Record::Commit {
            epoch: 1,
            data: encoding::encode(&first.commit),
        },
        Record::Welcome {
            epoch: 1,
            node: 2,
            data: encoding::encode(&first.welcome.clone().unwrap()),
        },
    ]);

    // The Welcome and its commit are held and not used, and not thrown away as refused.
    assert_eq!(bob_member.advance(), vec![]);
    assert!(bob_member.device().epoch().is_err(), "joined no group");

    // The entry reaches the host: now the Welcome is taken.
    bob_member.trust().append(&first_entry).unwrap();
    assert_eq!(bob_member.advance(), vec![Step::Joined(1)]);

    // The same for a commit of the group: held until the log names it.
    bob_member.absorb(&[Record::Commit {
        epoch: 2,
        data: encoding::encode(&second.commit),
    }]);
    assert_eq!(bob_member.advance(), vec![]);
    assert_eq!(bob_member.device().epoch(), Ok(1));

    bob_member.trust().append(&carol_grant).unwrap();
    bob_member.trust().append(&second_entry).unwrap();
    assert_eq!(bob_member.advance(), vec![Step::Applied(2)]);
    assert_eq!(
        bob_member.device().epoch_key().unwrap(),
        alice.epoch_key().unwrap()
    );
}
