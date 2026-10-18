// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// A relay that is not trusted gives a host the commits of the group in whatever order it likes, and it
// may give ones nobody made, before or after the one the log names for the epoch. The host keeps the
// commit whose hash the log names, whichever arrives first, and does not let what it was given stand in
// the way of the true one: a Welcome with the true commit joins the device whether the log named the
// commit before the relay gave any, or after. A host waits on a bounded number of commits for an epoch
// the log has not named one for; past the bound the true one can be dropped, and the host is told which
// commit it lacks (`commits_to_fetch`) so that it reads the relay again, and takes it.

use sce_rust_mls::host::{Issued, Member, Step};
use sce_rust_mls::Device;
use sce_rust_relay::Record;
use sce_rust_trust::encoding;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{Identity, Trust};

const LIST: &str = "home";

fn key(seed: u8) -> Identity {
    Identity::from_seed([seed; 32])
}

fn entries() -> Vec<Entry> {
    let scratch = Trust::open(LIST);
    let genesis = Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![key(9).public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&key(9))
    .finish();
    scratch.append(&genesis).unwrap();
    let grant = scratch
        .draft(Body::Grant {
            node: 1,
            role: 3,
            key: key(1).public(),
            final_seq: None,
        })
        .sign(&key(9))
        .finish();
    scratch.append(&grant).unwrap();
    scratch.entries()
}

fn host(entries: &[Entry], node: u64) -> Trust {
    let trust = Trust::invited(LIST, entries[0].hash());
    for entry in entries {
        trust.append(entry).unwrap();
    }
    trust.set_identity(node, key(node as u8));
    trust
}

/// What the administrator (node 1) makes when it adds bob (node 2): the commit and the Welcome, and
/// the log that names the commit; and bob's device, ready to join.
fn invitation() -> (Issued, Vec<Entry>, Device) {
    let log = entries();
    let mut alice = Device::new(LIST, 1, key(1));
    alice.create_group().unwrap();
    let mut bob = Device::new(LIST, 2, key(2));
    let package = bob.key_package().unwrap();
    let mut administrator = Member::new(alice, host(&log, 1), 1);
    let issued = administrator
        .add_member(&key(9), 2, 2, key(2).public(), &package)
        .unwrap();
    (issued, administrator.trust().entries(), bob)
}

fn commit(issued: &Issued, data: &[u8]) -> Record {
    Record::Commit {
        epoch: issued.epoch,
        data: encoding::encode(data),
    }
}

fn welcome(issued: &Issued) -> Record {
    Record::Welcome {
        epoch: issued.epoch,
        node: 2,
        data: encoding::encode(issued.welcome.as_ref().unwrap()),
    }
}

#[test]
fn a_false_commit_given_before_and_after_the_true_one_does_not_keep_a_device_from_it() {
    let (issued, log, bob) = invitation();
    let mut victim = Member::new(bob, host(&log, 2), 2);

    // The log names the commit already. A false one first, the true one, and another false one after
    // it, then the Welcome: the true one is the one kept.
    victim.absorb(&[
        commit(&issued, b"invented by the relay"),
        commit(&issued, &issued.commit),
        commit(&issued, b"invented after it"),
        welcome(&issued),
    ]);
    let steps = victim.advance();
    assert_eq!(
        steps,
        vec![Step::Joined(u64::from(issued.epoch))],
        "{steps:?}"
    );
    assert!(victim.device().epoch().is_ok());
}

#[test]
fn the_true_commit_among_many_false_ones_given_before_the_log_names_it_is_the_one_used() {
    // Each time a host that has the log up to its own grant, and not yet the entry that names the commit,
    // is given forty false commits, the true one and the Welcome; it waits, and refuses nothing. When the
    // entry comes it joins with the true one, whichever of them its table would have listed first.
    for _ in 0..3 {
        let (issued, log, bob) = invitation();
        let (named, before) = log.split_last().unwrap();
        let mut late = Member::new(bob, host(before, 2), 2);

        let mut given: Vec<Record> = (0u8..40).map(|n| commit(&issued, &[n; 12])).collect();
        given.push(commit(&issued, &issued.commit));
        given.push(welcome(&issued));
        late.absorb(&given);
        assert!(
            late.advance().is_empty(),
            "the log names no commit yet: the host waits, and refuses nothing"
        );
        assert!(late.device().epoch().is_err());

        late.trust().append(named).unwrap();
        assert_eq!(
            late.advance(),
            vec![Step::Joined(u64::from(issued.epoch))],
            "the true commit was among forty false ones"
        );
    }
}

/// A host that has the log up to its own grant is given `false_ones` false commits, the true one and
/// the Welcome, then the entry that names the commit.
fn waits_then_is_told(false_ones: usize) -> (Member, Vec<Record>, Issued) {
    let (issued, log, bob) = invitation();
    let (named, before) = log.split_last().unwrap();
    let mut late = Member::new(bob, host(before, 2), 2);

    let mut given: Vec<Record> = (0..false_ones)
        .map(|n| commit(&issued, format!("invented {n}").as_bytes()))
        .collect();
    given.push(commit(&issued, &issued.commit));
    given.push(welcome(&issued));
    late.absorb(&given);
    assert!(late.advance().is_empty());
    late.trust().append(named).unwrap();
    (late, given, issued)
}

#[test]
fn a_true_commit_that_fell_past_the_waiting_limit_is_told_and_taken_when_read_again() {
    // 63 false commits and the true one fill the limit exactly: the true one is kept.
    let (mut within, _, issued) = waits_then_is_told(63);
    assert!(within.commits_to_fetch().is_empty());
    assert_eq!(
        within.advance(),
        vec![Step::Joined(u64::from(issued.epoch))]
    );

    // 64 false commits first: the true one was over the limit and is gone, and the host says so
    // instead of staying quiet.
    let (mut over, given, issued) = waits_then_is_told(64);
    assert!(
        over.advance().is_empty(),
        "nothing is held that the log names"
    );
    assert!(over.device().epoch().is_err());
    assert_eq!(over.commits_to_fetch(), vec![issued.epoch]);

    // The relay is read again from the start: the log names the commit now, so the true one is kept
    // whatever the false ones before it, and the device joins.
    over.absorb(&given);
    assert!(over.commits_to_fetch().is_empty());
    assert_eq!(over.advance(), vec![Step::Joined(u64::from(issued.epoch))]);
}

#[test]
fn a_host_that_waits_for_nothing_is_told_to_fetch_nothing() {
    let (issued, log, bob) = invitation();
    let (named, before) = log.split_last().unwrap();
    let mut quiet = Member::new(bob, host(before, 2), 2);
    assert!(quiet.commits_to_fetch().is_empty(), "no Welcome is held");
    quiet.absorb(&[welcome(&issued)]);
    assert!(
        quiet.commits_to_fetch().is_empty(),
        "the log names no commit yet: there is nothing to ask for"
    );
    quiet.trust().append(named).unwrap();
    assert_eq!(quiet.commits_to_fetch(), vec![issued.epoch]);
}
