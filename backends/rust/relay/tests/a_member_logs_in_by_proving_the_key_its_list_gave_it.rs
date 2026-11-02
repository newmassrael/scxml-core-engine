// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A member's device logs in at a relay by proving the key its list's log gave its node, and the relay
// issues its token (ADR 0080 sections 1, 2 and 5). What is asked here: that a member's device, driven
// by the one driver over a link that presents no token, is issued a token the relay takes for its
// node and its list, good for an hour; that a login given again is refused, as one signed for another
// relay, one against a nonce made for another node, one whose challenge expired, and one of a member
// taken off; that a device configured for another relay than the one that answers stops before it
// signs anything; that a device brings the grant the relay was not given yet, and without it is no
// member; that a list the relay holds no log of is not made by a login; that a source is held to so
// many connections, challenges and logins a minute and a node to so many logins an hour; that what a
// connection with no token may ask besides is refused, and a relay that offers no login says so; and
// that a relay of several slots answers a login as a relay of one does.

use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use sce_rust_relay::{
    drive, Authority, Code, DirectLink, Doorway, Hub, Limits, Login, LoginMachine, LoginPolicy,
    MemoryStore, Opened, Relay, Request, Response, Session, Shards, Signal,
};
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{encoding, Identity, Trust};

const LIST: &str = "home";
const AUDIENCE: &str = "relay-a";
const ADMIN: u8 = 9;
/// The relay's clock when each test begins.
const NOW: u64 = 1_000_000;

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn admin() -> Identity {
    Identity::from_seed([ADMIN; 32])
}

/// The log of the list: alice (1) and bob (2) write, carol (3) reads; and a host that holds it.
fn log() -> (Vec<Entry>, Trust) {
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
    (all, scratch)
}

fn entries() -> Vec<Entry> {
    log().0
}

/// The host of the device of `node`, which holds the log and its person's key.
fn device(node: u64) -> Trust {
    let trust = Trust::open(LIST);
    for entry in entries() {
        trust.append(&entry).unwrap();
    }
    trust.set_identity(node, person(node));
    trust
}

fn authority() -> Authority {
    Authority::new([7; 32], AUDIENCE)
}

fn policy() -> LoginPolicy {
    LoginPolicy::default()
}

/// A relay that offers login under `policy`, given the first `given` entries of the log by an
/// operator's token.
fn relay(policy: LoginPolicy, given: usize) -> Relay<MemoryStore> {
    let mut relay = Relay::open(MemoryStore::new(), authority(), Limits::default())
        .unwrap()
        .with_login(Doorway::with_key(policy, [3; 32]));
    let operator = authority().issue(LIST, 1, u64::MAX);
    for entry in entries().into_iter().take(given) {
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
    relay
}

fn anonymous(relay: &mut Relay<MemoryStore>, request: &Request, now: u64) -> Response {
    Response::from_json(&relay.anonymous(None, &request.to_json(), now)).unwrap()
}

/// A challenge for `node`: its nonce and the instant it expires at.
fn challenge(relay: &mut Relay<MemoryStore>, node: u64, now: u64) -> (String, u64) {
    match anonymous(
        relay,
        &Request::Challenge {
            list: LIST.to_string(),
            node,
        },
        now,
    ) {
        Response::Challenge {
            audience,
            nonce,
            expires_at_ms,
            held,
            work,
        } => {
            assert_eq!(audience, AUDIENCE);
            assert!(held);
            assert_eq!(work, None);
            (nonce, expires_at_ms)
        }
        other => panic!("a challenge, and not {other:?}"),
    }
}

/// The login of the device of `node`, signed by `signer` for `audience`, against a challenge.
fn signed(
    node: u64,
    signer: u64,
    audience: &str,
    (nonce, expires_at_ms): &(String, u64),
    entries: Vec<String>,
) -> Request {
    let bytes = sce_rust_trust::login::signing_bytes(
        audience,
        LIST,
        node,
        &encoding::decode(nonce).unwrap(),
        *expires_at_ms,
    );
    Request::Login(Login {
        list: LIST.to_string(),
        node,
        nonce: nonce.clone(),
        expires_at_ms: *expires_at_ms,
        signature: encoding::encode(&person(signer).sign(&bytes)),
        entries,
    })
}

fn refused(response: &Response) -> (Code, String) {
    match response {
        Response::Refused { code, reason } => (code.clone(), reason.clone()),
        other => panic!("a refusal, and not {other:?}"),
    }
}

fn token_of(response: &Response) -> (String, u64) {
    match response {
        Response::Token {
            token,
            expires_at_ms,
        } => (token.clone(), *expires_at_ms),
        other => panic!("a token, and not {other:?}"),
    }
}

#[test]
fn a_member_logs_in_over_a_link_with_no_token_and_is_issued_one_the_relay_takes() {
    let relay = Arc::new(Mutex::new(relay(policy(), 4)));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    let mut login = LoginMachine::start(device(2), AUDIENCE);
    drive(&mut login, &mut link, None, &Signal::new());
    let token = login
        .outcome()
        .cloned()
        .expect("the login is over")
        .expect("the login came to a token");
    assert_eq!(token.expires_at_ms, NOW + 3_600_000, "a token of an hour");
    assert_eq!(
        authority().verify(&token.token, NOW),
        Ok(Session {
            list: LIST.to_string(),
            node: 2
        })
    );
    // The link presents it from now on, and the relay answers the member's reading.
    assert!(sce_rust_relay::Transport::set_token(
        &mut link,
        &token.token
    ));
    let heads = sce_rust_relay::Link::call(&mut link, &Request::Heads).unwrap();
    assert!(
        matches!(heads, Response::Heads { log_len: 4, .. }),
        "{heads:?}"
    );
}

#[test]
fn a_login_given_again_is_refused() {
    let mut relay = relay(policy(), 4);
    let asked = challenge(&mut relay, 2, NOW);
    let login = signed(2, 2, AUDIENCE, &asked, Vec::new());
    token_of(&anonymous(&mut relay, &login, NOW + 10));
    let (code, reason) = refused(&anonymous(&mut relay, &login, NOW + 20));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("used"), "{reason}");
}

#[test]
fn a_login_signed_for_another_relay_is_refused_and_a_device_configured_for_another_stops() {
    let mut relay = relay(policy(), 4);
    // Signed for another relay's name, which a relay that forwards another's challenge would get.
    let asked = challenge(&mut relay, 2, NOW);
    let (code, reason) = refused(&anonymous(
        &mut relay,
        &signed(2, 2, "relay-b", &asked, Vec::new()),
        NOW,
    ));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("not signed"), "{reason}");

    // A device configured for another relay than the one that answers signs nothing for it.
    let relay = Arc::new(Mutex::new(relay));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let mut link = DirectLink::new(relay, String::new(), clock);
    let mut login = LoginMachine::start(device(2), "relay-b");
    drive(&mut login, &mut link, None, &Signal::new());
    let stopped = login.outcome().cloned().unwrap().unwrap_err();
    assert!(format!("{stopped:?}").contains("relay-b"), "{stopped:?}");
}

#[test]
fn a_nonce_made_for_another_node_is_no_use_for_this_one() {
    let mut relay = relay(policy(), 4);
    let for_alice = challenge(&mut relay, 1, NOW);
    let (code, reason) = refused(&anonymous(
        &mut relay,
        &signed(2, 2, AUDIENCE, &for_alice, Vec::new()),
        NOW,
    ));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("this list and node"), "{reason}");
    // Nor is a nonce of another relay's key, nor one changed by a bit.
    let mut other = Relay::open(MemoryStore::new(), authority(), Limits::default())
        .unwrap()
        .with_login(Doorway::with_key(policy(), [4; 32]));
    let theirs = match anonymous(
        &mut other,
        &Request::Challenge {
            list: LIST.to_string(),
            node: 2,
        },
        NOW,
    ) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            ..
        } => (nonce, expires_at_ms),
        other => panic!("{other:?}"),
    };
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(2, 2, AUDIENCE, &theirs, Vec::new()),
        NOW,
    ));
    assert_eq!(code, Code::Unauthorized);
    let ours = challenge(&mut relay, 2, NOW);
    let later = (ours.0.clone(), ours.1 + 1);
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(2, 2, AUDIENCE, &later, Vec::new()),
        NOW,
    ));
    assert_eq!(code, Code::Unauthorized, "the instant is under the tag");
}

#[test]
fn a_challenge_that_expired_is_refused() {
    let mut relay = relay(policy(), 4);
    let asked = challenge(&mut relay, 2, NOW);
    assert_eq!(asked.1, NOW + 60_000, "a minute by the relay's clock");
    let login = signed(2, 2, AUDIENCE, &asked, Vec::new());
    let (code, reason) = refused(&anonymous(&mut relay, &login, asked.1));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("expired"), "{reason}");
    token_of(&anonymous(&mut relay, &login, asked.1 - 1));
}

#[test]
fn a_member_taken_off_is_issued_no_token_and_a_stranger_none() {
    let (all, scratch) = log();
    let removal = scratch
        .draft(scratch.removal_body(2).expect("bob is a member"))
        .sign(&admin())
        .finish();
    let mut relay = relay(policy(), 4);
    let operator = authority().issue(LIST, 1, u64::MAX);
    let answer = relay.handle(
        &operator,
        &Request::Append {
            entry: removal.to_json(),
        }
        .to_json(),
        NOW,
    );
    assert!(
        answer.contains("\"appended\""),
        "{answer} after {}",
        all.len()
    );
    for node in [2, 4] {
        let asked = challenge(&mut relay, node, NOW);
        let (code, reason) = refused(&anonymous(
            &mut relay,
            &signed(node, node, AUDIENCE, &asked, Vec::new()),
            NOW,
        ));
        assert_eq!(code, Code::Forbidden, "node {node}");
        assert!(reason.contains("no member"), "{reason}");
    }
    // Signed by alice's key for carol's node: the key the log gives the node is the one that holds.
    let asked = challenge(&mut relay, 3, NOW);
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(3, 1, AUDIENCE, &asked, Vec::new()),
        NOW,
    ));
    assert_eq!(code, Code::Unauthorized);
}

#[test]
fn a_device_brings_the_grant_the_relay_was_not_given() {
    // The relay holds the genesis and alice's grant; bob's is not given it yet.
    let mut relay = relay(policy(), 2);
    let asked = challenge(&mut relay, 2, NOW);
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(2, 2, AUDIENCE, &asked, Vec::new()),
        NOW,
    ));
    assert_eq!(
        code,
        Code::Forbidden,
        "no member, by the log the relay holds"
    );
    // Brought with the login, the grant is appended first, and the login holds; an entry the relay
    // holds already is passed over.
    let brought = entries()[1..3]
        .iter()
        .map(Entry::to_json)
        .collect::<Vec<_>>();
    let asked = challenge(&mut relay, 2, NOW);
    token_of(&anonymous(
        &mut relay,
        &signed(2, 2, AUDIENCE, &asked, brought),
        NOW,
    ));
    assert_eq!(relay.records(LIST), 3, "the genesis and two grants");
    // A grant past what the relay holds, with a gap before it, is `behind`.
    let mut relay = self::relay(policy(), 2);
    let asked = challenge(&mut relay, 3, NOW);
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(3, 3, AUDIENCE, &asked, vec![entries()[3].to_json()]),
        NOW,
    ));
    assert_eq!(code, Code::Behind);
    // The device's own side brings the entry of its latest grant: bob's follows alice's, which the
    // relay holds, so the relay takes it and bob is a member by it.
    let relay = Arc::new(Mutex::new(self::relay(policy(), 2)));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| NOW);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    let mut login = LoginMachine::start(device(2), AUDIENCE);
    drive(&mut login, &mut link, None, &Signal::new());
    let token = login.outcome().cloned().unwrap().expect("a token");
    assert_eq!(
        authority()
            .verify(&token.token, NOW)
            .map(|session| session.node),
        Ok(2)
    );
    assert_eq!(
        relay.lock().unwrap().records(LIST),
        3,
        "bob's grant was kept"
    );
}

#[test]
fn a_login_makes_no_list_the_relay_holds_no_log_of() {
    let mut relay = relay(policy(), 0);
    let asked = match anonymous(
        &mut relay,
        &Request::Challenge {
            list: LIST.to_string(),
            node: 1,
        },
        NOW,
    ) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            held,
            ..
        } => {
            assert!(!held, "the relay holds no such list");
            (nonce, expires_at_ms)
        }
        other => panic!("{other:?}"),
    };
    let genesis = vec![entries()[0].to_json(), entries()[1].to_json()];
    let (code, _) = refused(&anonymous(
        &mut relay,
        &signed(1, 1, AUDIENCE, &asked, genesis),
        NOW,
    ));
    assert_eq!(code, Code::Behind);
    assert_eq!(relay.records(LIST), 0, "nothing was kept");
}

#[test]
fn a_source_and_a_node_are_held_to_their_allowances() {
    let tight = LoginPolicy {
        connections_per_minute: 2,
        challenges_per_minute: 2,
        logins_per_minute: 3,
        logins_per_node_per_hour: 1,
        ..policy()
    };
    let mut relay = relay(tight, 4);
    let here: IpAddr = "192.0.2.7".parse().unwrap();
    let there: IpAddr = "2001:db8:1:2::9".parse().unwrap();
    let neighbour: IpAddr = "2001:db8:1:2::10".parse().unwrap();

    assert_eq!(relay.open_anonymous(Some(here), NOW), Opened::Taken);
    assert_eq!(relay.open_anonymous(Some(here), NOW), Opened::Taken);
    assert!(matches!(
        relay.open_anonymous(Some(here), NOW),
        Opened::RateLimited { .. }
    ));
    assert_eq!(relay.open_anonymous(Some(there), NOW), Opened::Taken);
    assert_eq!(relay.open_anonymous(Some(neighbour), NOW), Opened::Taken);
    assert!(
        matches!(
            relay.open_anonymous(Some(neighbour), NOW),
            Opened::RateLimited { .. }
        ),
        "one /64 is one source"
    );
    assert_eq!(
        relay.open_anonymous(Some(here), NOW + 60_000),
        Opened::Taken,
        "a minute later"
    );

    let ask = |relay: &mut Relay<MemoryStore>, source: IpAddr, request: &Request| {
        Response::from_json(&relay.anonymous(Some(source), &request.to_json(), NOW)).unwrap()
    };
    let challenge_of = |node: u64| Request::Challenge {
        list: LIST.to_string(),
        node,
    };
    let first = match ask(&mut relay, here, &challenge_of(2)) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            ..
        } => (nonce, expires_at_ms),
        other => panic!("{other:?}"),
    };
    let second = match ask(&mut relay, here, &challenge_of(2)) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            ..
        } => (nonce, expires_at_ms),
        other => panic!("{other:?}"),
    };
    let (code, _) = refused(&ask(&mut relay, here, &challenge_of(2)));
    assert!(matches!(code, Code::RateLimited { .. }), "{code:?}");

    // A login that fails spends its source's allowance and not the node's.
    let (code, _) = refused(&ask(
        &mut relay,
        there,
        &signed(2, 1, AUDIENCE, &first, Vec::new()),
    ));
    assert_eq!(code, Code::Unauthorized);
    token_of(&ask(
        &mut relay,
        there,
        &signed(2, 2, AUDIENCE, &first, Vec::new()),
    ));
    // The node logged in as often as it may this hour.
    let (code, reason) = refused(&ask(
        &mut relay,
        there,
        &signed(2, 2, AUDIENCE, &second, Vec::new()),
    ));
    assert!(matches!(code, Code::RateLimited { .. }), "{code:?}");
    assert!(reason.contains("this hour"), "{reason}");
    let (code, reason) = refused(&ask(
        &mut relay,
        there,
        &signed(2, 2, AUDIENCE, &second, Vec::new()),
    ));
    assert!(matches!(code, Code::RateLimited { .. }), "{code:?}");
    assert!(reason.contains("source"), "three logins a minute: {reason}");
}

#[test]
fn a_connection_with_no_token_asks_for_a_challenge_and_logs_in_and_nothing_else() {
    let mut relay = relay(policy(), 4);
    let (code, _) = refused(&anonymous(&mut relay, &Request::Heads, NOW));
    assert_eq!(code, Code::Unauthorized);
    let create = r#"{"v":1,"op":"create","list":"home","node":"1"}"#;
    let refused_create = Response::from_json(&relay.anonymous(None, create, NOW)).unwrap();
    let (code, reason) = refused(&refused_create);
    assert_eq!(code, Code::Invalid);
    assert!(reason.contains("`create`"), "{reason}");
    let join = r#"{"v":1,"op":"join"}"#;
    let (code, reason) = refused(&Response::from_json(&relay.anonymous(None, join, NOW)).unwrap());
    assert_eq!(code, Code::Invalid);
    assert!(reason.contains("`join`"), "{reason}");

    // A session's token names its list already: a login is not a session's request.
    let token = authority().issue(LIST, 2, u64::MAX);
    let asked = challenge(&mut relay, 2, NOW);
    let login = signed(2, 2, AUDIENCE, &asked, Vec::new());
    let answer = relay.handle(&token, &login.to_json(), NOW);
    assert_eq!(
        refused(&Response::from_json(&answer).unwrap()).0,
        Code::Invalid
    );

    // A relay that offers no login says so, and takes no connection with no token.
    let mut closed = Relay::open(MemoryStore::new(), authority(), Limits::default()).unwrap();
    assert_eq!(closed.open_anonymous(None, NOW), Opened::NotOffered);
    let answer = closed.anonymous(
        None,
        &Request::Challenge {
            list: LIST.to_string(),
            node: 2,
        }
        .to_json(),
        NOW,
    );
    let (code, reason) = refused(&Response::from_json(&answer).unwrap());
    assert_eq!(code, Code::Invalid);
    assert!(reason.contains("does not offer login"), "{reason}");
    let answer = closed.anonymous(None, &Request::Heads.to_json(), NOW);
    assert_eq!(
        refused(&Response::from_json(&answer).unwrap()).0,
        Code::Unauthorized
    );
}

#[test]
fn a_relay_of_several_slots_issues_a_token_as_a_relay_of_one_does() {
    let shards = Shards::open(
        4,
        |_| Ok(MemoryStore::new()),
        authority(),
        Limits::default(),
    )
    .unwrap()
    .with_login(Doorway::with_key(policy(), [3; 32]));
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
    let challenge = Request::Challenge {
        list: LIST.to_string(),
        node: 2,
    };
    // A request to log in is answered on a connection that presented a token too.
    let asked = match Response::from_json(&shards.answer(
        Some(operator.as_str()),
        None,
        &challenge.to_json(),
        NOW,
    ))
    .unwrap()
    {
        Response::Challenge {
            nonce,
            expires_at_ms,
            held,
            ..
        } => {
            assert!(held);
            (nonce, expires_at_ms)
        }
        other => panic!("{other:?}"),
    };
    let login = signed(2, 2, AUDIENCE, &asked, Vec::new());
    let (token, _) =
        token_of(&Response::from_json(&shards.answer(None, None, &login.to_json(), NOW)).unwrap());
    assert_eq!(
        shards.authenticate(&token, NOW).map(|session| session.node),
        Ok(2)
    );
    let again = Response::from_json(&shards.answer(None, None, &login.to_json(), NOW)).unwrap();
    assert_eq!(
        refused(&again).0,
        Code::Unauthorized,
        "used once, in any slot"
    );
    assert_eq!(shards.open_anonymous(None, NOW), Opened::Taken);
}
