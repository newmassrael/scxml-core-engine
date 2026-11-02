// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A device that is not in the list's log leaves its key package with the relay under an
// administrator's invitation (ADR 0080 section 4). What is asked here: that a device driven by the
// one driver over a link that presents no token joins, and the relay keeps its key package with the
// invitation and the proof, which hold for anyone who reads the record; that an invitation is used
// once, and a second join of its node is refused; that an invitation that expired, one for a node
// the log granted, one signed by a key that is no administrator's, one for another relay and a proof
// under another key than the invitation's are each refused; that a relay read again from its store
// holds the join, as the record of the invitation's use; that a list holds so many joins waiting for
// an administrator; and that a relay of several slots answers a join as a relay of one does.

use std::io;
use std::sync::{Arc, Mutex};

use sce_rust_relay::{
    drive, Authority, Code, CourierError, DirectLink, Doorway, Hub, Join, JoinMachine, Limits,
    LoginPolicy, MemoryStore, Record, Relay, Request, Response, Shards, Signal, Store,
};
use sce_rust_trust::invitation::{self, Invitation, Proof, Ticket};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{encoding, Identity, Trust};

const LIST: &str = "home";
const AUDIENCE: &str = "relay-a";
const ADMIN: u8 = 9;
/// The relay's clock when each test begins.
const NOW: u64 = 1_000_000;
/// The node the invitations are for: one the log never granted.
const NEW: u64 = 5;
const SECRET: [u8; 32] = [4; 32];

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([ADMIN; 32])
}

/// The log of the list: alice (1) and bob (2) write, carol (3) reads.
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
    let mut all = vec![genesis];
    for (node, role) in [(1u64, 2u8), (2, 2), (3, 1)] {
        let entry = scratch
            .draft(Body::Grant {
                node,
                role,
                key: person(node).public(),
                final_seq: None,
            })
            .sign(&admin())
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

fn authority() -> Authority {
    Authority::new([7; 32], AUDIENCE)
}

/// A store whose records outlive the relay that wrote them, so that another relay reads them again.
#[derive(Clone, Default)]
struct Kept(Arc<Mutex<Vec<(String, Record)>>>);

impl Store for Kept {
    fn load(&mut self) -> io::Result<Vec<(String, Record)>> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn append(&mut self, list: &str, record: &Record) -> io::Result<()> {
        self.0
            .lock()
            .unwrap()
            .push((list.to_string(), record.clone()));
        Ok(())
    }
}

fn give_log<S: Store>(relay: &mut Relay<S>) {
    let operator = authority().issue(LIST, 1, u64::MAX);
    for entry in entries() {
        let answer = relay.handle(
            &operator,
            &Request::Append {
                entry: entry.to_json(),
            }
            .to_json(),
            NOW,
        );
        assert!(answer.contains("\"appended\""), "{answer}");
    }
}

/// A relay that offers login and joins under `policy`, given the list's log by an operator's token.
fn relay_over(store: Kept, policy: LoginPolicy) -> Relay<Kept> {
    let mut relay = Relay::open(store, authority(), Limits::default())
        .unwrap()
        .with_login(Doorway::with_key(policy, [3; 32]));
    give_log(&mut relay);
    relay
}

fn relay() -> Relay<Kept> {
    relay_over(Kept::default(), LoginPolicy::default())
}

/// The invitation `signer` makes of the device of `node` at the relay named `audience`, until
/// `expires_at_ms`, under the secret `secret`.
fn ticket_by(
    signer: &Identity,
    audience: &str,
    node: u64,
    expires_at_ms: u64,
    secret: [u8; 32],
) -> Ticket {
    Ticket {
        relay: "wss://relay.example".to_string(),
        invitation: Invitation::sign(
            signer,
            audience,
            LIST,
            entries()[0].hash(),
            node,
            expires_at_ms,
            invitation::key_of(&secret).public(),
        ),
        secret,
    }
}

/// The administrator's invitation of the device of [`NEW`], good for a day.
fn ticket() -> Ticket {
    ticket_by(&admin(), AUDIENCE, NEW, NOW + invitation::MOST_MS, SECRET)
}

fn anonymous<S: Store>(relay: &mut Relay<S>, request: &Request, now: u64) -> Response {
    Response::from_json(&relay.anonymous(None, &request.to_json(), now)).unwrap()
}

/// The join of the device that signs with `key` and leaves `package`, under `ticket`, its proof
/// made with `proof_secret`: a challenge for the invitation's node, and the join against it.
fn join_with<S: Store>(
    relay: &mut Relay<S>,
    ticket: &Ticket,
    proof_secret: [u8; 32],
    key: &Identity,
    package: &[u8],
) -> Response {
    let invitation = &ticket.invitation;
    let challenge = anonymous(
        relay,
        &Request::Challenge {
            list: invitation.list.clone(),
            node: invitation.node,
        },
        NOW,
    );
    let Response::Challenge {
        nonce,
        expires_at_ms,
        ..
    } = challenge
    else {
        panic!("a challenge, and not {challenge:?}")
    };
    let proof = Proof::make(
        &proof_secret,
        invitation,
        &encoding::decode(&nonce).unwrap(),
        expires_at_ms,
        key.public(),
        package,
    );
    let request = Request::Join(Box::new(Join {
        invitation: invitation.clone(),
        proof,
        key_package: (!package.is_empty()).then(|| encoding::encode(package)),
    }));
    anonymous(relay, &request, NOW)
}

fn join<S: Store>(relay: &mut Relay<S>, ticket: &Ticket, key: &Identity) -> Response {
    join_with(relay, ticket, ticket.secret, key, b"a key package")
}

fn refused(response: &Response) -> (Code, String) {
    match response {
        Response::Refused { code, reason } => (code.clone(), reason.clone()),
        other => panic!("a refusal, and not {other:?}"),
    }
}

/// The records the relay holds of the list, read with an operator's token.
fn records(relay: &Mutex<Relay<Kept>>) -> Vec<Record> {
    let operator = authority().issue(LIST, 1, u64::MAX);
    let answer = relay.handle(
        &operator,
        &Request::Feed {
            cursor: 0,
            limit: 500,
        }
        .to_json(),
        NOW,
    );
    match Response::from_json(&answer).unwrap() {
        Response::Feed { records, .. } => records,
        other => panic!("a feed, and not {other:?}"),
    }
}

#[test]
fn a_device_joins_over_a_link_with_no_token_and_the_relay_keeps_its_key_package_with_the_proof() {
    let relay = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    let device = person(NEW);
    let mut joining = JoinMachine::start(ticket(), device.public(), b"a key package".to_vec());
    drive(&mut joining, &mut link, None, &Signal::new());
    assert_eq!(joining.outcome(), Some(&Ok(())), "the join was accepted");

    let kept = records(&relay);
    let Some(Record::KeyPackage {
        node,
        data,
        join: Some(join),
    }) = kept.last()
    else {
        panic!("the last record is the key package left under the invitation: {kept:?}")
    };
    assert_eq!(*node, NEW);
    assert_eq!(encoding::decode(data).unwrap(), b"a key package");
    assert_eq!(join.invitation, ticket().invitation);
    assert_eq!(
        join.proof.key,
        device.public(),
        "the identity key it proved"
    );
    assert!(
        join.proof.holds(&join.invitation, b"a key package"),
        "anyone who reads the record checks the proof under the invitation's key"
    );
}

#[test]
fn a_device_with_no_key_package_joins_with_its_key_alone() {
    let mut relay = relay();
    let answer = join_with(&mut relay, &ticket(), SECRET, &person(NEW), &[]);
    assert!(
        matches!(answer, Response::Accepted { new: true, .. }),
        "{answer:?}"
    );
    let kept = records(&Mutex::new(relay));
    assert!(
        matches!(kept.last(), Some(Record::KeyPackage { data, join: Some(_), .. }) if data.is_empty()),
        "{kept:?}"
    );
}

#[test]
fn an_invitation_is_used_once() {
    let mut relay = relay();
    let first = join(&mut relay, &ticket(), &person(NEW));
    assert!(matches!(first, Response::Accepted { .. }), "{first:?}");
    // The same invitation again, by the device or by someone who photographed its code, with
    // another key: the key package kept for the node is the record of its use.
    let (code, reason) = refused(&join(&mut relay, &ticket(), &person(6)));
    assert_eq!(code, Code::Forbidden);
    assert!(reason.contains("was used"), "{reason}");
    // And another invitation of the same node: the node's invitation was used.
    let again = ticket_by(&admin(), AUDIENCE, NEW, NOW + 60_000, [8; 32]);
    let (code, _) = refused(&join(&mut relay, &again, &person(NEW)));
    assert_eq!(code, Code::Forbidden);
}

#[test]
fn an_invitation_that_expired_is_refused() {
    let mut relay = relay();
    let lapsed = ticket_by(&admin(), AUDIENCE, NEW, NOW, SECRET);
    let (code, reason) = refused(&join(&mut relay, &lapsed, &person(NEW)));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("expired"), "{reason}");
}

#[test]
fn an_invitation_for_a_node_the_log_granted_is_refused() {
    let mut relay = relay();
    // Carol's node holds a grant: an invitation is for a node the log never granted.
    let granted = ticket_by(&admin(), AUDIENCE, 3, NOW + 60_000, SECRET);
    let (code, reason) = refused(&join(&mut relay, &granted, &person(3)));
    assert_eq!(code, Code::Forbidden);
    assert!(reason.contains("granted"), "{reason}");
}

#[test]
fn an_invitation_no_administrator_signed_is_refused() {
    let mut relay = relay();
    // Alice is a member who writes, and no administrator: she cannot invite.
    let forged = ticket_by(&person(1), AUDIENCE, NEW, NOW + 60_000, SECRET);
    let (code, reason) = refused(&join(&mut relay, &forged, &person(NEW)));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("administrator"), "{reason}");
    // An administrator's invitation changed after it was signed holds no signature.
    let mut changed = ticket();
    changed.invitation.expires_at_ms += 1;
    let (code, reason) = refused(&join(&mut relay, &changed, &person(NEW)));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("signature"), "{reason}");
}

#[test]
fn an_invitation_for_another_relay_is_refused() {
    let mut relay = relay();
    let elsewhere = ticket_by(&admin(), "relay-b", NEW, NOW + 60_000, SECRET);
    let (code, reason) = refused(&join(&mut relay, &elsewhere, &person(NEW)));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("another relay"), "{reason}");
}

#[test]
fn a_proof_under_another_key_than_the_invitations_is_refused() {
    let mut relay = relay();
    // What a relay that knew the invitation's public key alone, or anyone without its secret,
    // makes: a proof under a key of its own.
    let answer = join_with(
        &mut relay,
        &ticket(),
        [8; 32],
        &person(NEW),
        b"a key package",
    );
    let (code, reason) = refused(&answer);
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("proof"), "{reason}");
    // Nothing was kept, so the invitation is still there to be used by the device.
    let first = join(&mut relay, &ticket(), &person(NEW));
    assert!(matches!(first, Response::Accepted { .. }), "{first:?}");
}

#[test]
fn a_relay_read_again_from_its_store_holds_the_join_as_the_record_of_its_use() {
    let store = Kept::default();
    let mut first = relay_over(store.clone(), LoginPolicy::default());
    let answer = join(&mut first, &ticket(), &person(NEW));
    assert!(matches!(answer, Response::Accepted { .. }), "{answer:?}");
    drop(first);

    // The relay starts again from its store: every record is read back through the checks that
    // took it in, the join among them, and the invitation stays used.
    let mut again = Relay::open(store, authority(), Limits::default())
        .expect("a store that holds a join reads back")
        .with_login(Doorway::with_key(LoginPolicy::default(), [3; 32]));
    let (code, reason) = refused(&join(&mut again, &ticket(), &person(6)));
    assert_eq!(code, Code::Forbidden);
    assert!(reason.contains("was used"), "{reason}");
}

#[test]
fn a_list_holds_so_many_joins_waiting_for_an_administrator() {
    let policy = LoginPolicy {
        joins_outstanding_per_list: 1,
        ..LoginPolicy::default()
    };
    let mut relay = relay_over(Kept::default(), policy);
    let first = join(&mut relay, &ticket(), &person(NEW));
    assert!(matches!(first, Response::Accepted { .. }), "{first:?}");
    let next = ticket_by(&admin(), AUDIENCE, 6, NOW + 60_000, [8; 32]);
    let (code, _) = refused(&join(&mut relay, &next, &person(6)));
    assert_eq!(code, Code::TooLarge);
}

#[test]
fn a_relay_of_several_slots_answers_a_join_as_a_relay_of_one_does() {
    let shards = Shards::open(
        4,
        |_| Ok(MemoryStore::new()),
        authority(),
        Limits::default(),
    )
    .unwrap()
    .with_login(Doorway::with_key(LoginPolicy::default(), [3; 32]));
    let operator = authority().issue(LIST, 1, u64::MAX);
    for entry in entries() {
        let answer = shards.handle(
            &operator,
            &Request::Append {
                entry: entry.to_json(),
            }
            .to_json(),
            NOW,
        );
        assert!(answer.contains("\"appended\""), "{answer}");
    }
    let mut joining = JoinMachine::start(ticket(), person(NEW).public(), b"a key package".to_vec());
    let outcome = loop {
        match joining.poll() {
            sce_rust_relay::JoinStep::Request(request) => {
                let answer = shards.answer(None, None, &request.to_json(), NOW);
                joining.respond(Ok(Response::from_json(&answer).unwrap()));
            }
            sce_rust_relay::JoinStep::Done(outcome) => break outcome,
        }
    };
    assert_eq!(outcome, Ok(()));
    let mut twice = JoinMachine::start(ticket(), person(6).public(), Vec::new());
    let outcome = loop {
        match twice.poll() {
            sce_rust_relay::JoinStep::Request(request) => {
                let answer = shards.answer(None, None, &request.to_json(), NOW);
                twice.respond(Ok(Response::from_json(&answer).unwrap()));
            }
            sce_rust_relay::JoinStep::Done(outcome) => break outcome,
        }
    };
    assert!(
        matches!(
            outcome,
            Err(CourierError::Relay {
                code: Code::Forbidden,
                ..
            })
        ),
        "{outcome:?}"
    );
}

#[test]
fn a_device_stops_before_it_proves_anything_to_a_relay_that_names_itself_otherwise() {
    // The relay answers `relay-a`; the invitation is for `relay-b`: the device signs nothing.
    let relay = Arc::new(Mutex::new(relay()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    let elsewhere = ticket_by(&admin(), "relay-b", NEW, NOW + 60_000, SECRET);
    let mut joining = JoinMachine::start(elsewhere, person(NEW).public(), Vec::new());
    drive(&mut joining, &mut link, None, &Signal::new());
    assert!(
        matches!(joining.outcome(), Some(Err(CourierError::Unexpected(_)))),
        "{:?}",
        joining.outcome()
    );
    let kept = records(&relay);
    assert!(
        !kept
            .iter()
            .any(|record| matches!(record, Record::KeyPackage { .. })),
        "{kept:?}"
    );
}
