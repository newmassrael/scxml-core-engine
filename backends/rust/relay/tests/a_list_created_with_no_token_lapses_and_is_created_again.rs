// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A relay that takes a creation from a connection with no token (ADR 0080 section 3), and the lapse
// of the lists so created. What is asked here: that a device whose list the relay does not hold is
// told the work a creation takes, and its login creates the list under a name that certifies the
// genesis, the relay keeping the log as far as the device's grant and issuing its token; that a
// creation is refused with too little work, under a name that does not certify its genesis, past
// the relay's lists, past a source's creations of the minute and past the relay's of the hour, and
// by a relay that takes none; that a lapse removes the lists created so that no login touched for
// their days and none other, a list created with a token of the authority never; that a login
// keeps a list alive, across a restart too; that a device that holds a list that lapsed creates it
// again with the same genesis and name, and gives it back what it holds; and that a store read
// again holds the mark of a list created so, and drops one a crash left with no list.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sce_rust_relay::client::{keep, CourierState};
use sce_rust_relay::{
    drive, Authority, Code, CourierError, Create, CreationPolicy, DirectLink, Doorway, FileStore,
    Hub, Kept, Limits, Login, LoginMachine, LoginPolicy, MemoryStore, Openly, Relay, Request,
    Response, Round, RoundKind, RoundMachine, Shards, Signal, Step, Store, Token, TOUCH_STEP_MS,
};
use sce_rust_trust::creation;
use sce_rust_trust::membership::{Body, Entry, Log, Profile};
use sce_rust_trust::{encoding, Identity, Trust};

const AUDIENCE: &str = "relay-a";
/// The relay's clock when each test begins.
const NOW: u64 = 1_000_000;
/// The work these tests ask: enough to be found, few enough to be quick.
const BITS: u32 = 8;
const DAY_MS: u64 = 86_400_000;
/// How long a list created openly may go untouched: the decision's 90 days.
const IDLE_MS: u64 = 90 * DAY_MS;

fn admin() -> Identity {
    Identity::from_seed([9; 32])
}

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn authority() -> Authority {
    Authority::new([7; 32], AUDIENCE)
}

/// The name of the list: the one that certifies its administrator's key.
fn list() -> String {
    creation::list_name(b"household", &admin().public()).unwrap()
}

/// The log of `list` under the administrator `administrator`: alice (1) and bob (2) write, carol
/// (3) reads.
fn log_of(list: &str, administrator: &Identity) -> Vec<Entry> {
    let scratch = Trust::open(list);
    let genesis = Log::genesis_draft(
        list,
        Body::Genesis {
            administrators: vec![administrator.public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(administrator)
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
            .sign(administrator)
            .finish();
        scratch.append(&entry).unwrap();
        all.push(entry);
    }
    all
}

fn entries() -> Vec<Entry> {
    log_of(&list(), &admin())
}

/// The host of the device of `node` of `list`, which holds `log` and its person's key.
fn device_of(list: &str, log: &[Entry], node: u64) -> Trust {
    let trust = Trust::open(list);
    for entry in log {
        trust.append(entry).unwrap();
    }
    trust.set_identity(node, person(node));
    trust
}

fn device(node: u64) -> Trust {
    device_of(&list(), &entries(), node)
}

fn creation_policy() -> CreationPolicy {
    CreationPolicy {
        work_bits: BITS,
        ..CreationPolicy::default()
    }
}

fn doorway(creation: Option<CreationPolicy>) -> Doorway {
    let doorway = Doorway::with_key(LoginPolicy::default(), [3; 32]);
    match creation {
        Some(policy) => doorway.with_creation(policy),
        None => doorway,
    }
}

/// A relay over `store` that offers login, and takes creations with no token under `creation`.
fn relay_over<S: Store>(store: S, creation: Option<CreationPolicy>, limits: Limits) -> Relay<S> {
    Relay::open(store, authority(), limits)
        .unwrap()
        .with_login(doorway(creation))
}

fn relay() -> Relay<MemoryStore> {
    relay_over(
        MemoryStore::new(),
        Some(creation_policy()),
        Limits::default(),
    )
}

/// The login of `trust`'s person at the relay `relay` at `now`, over a link that presents no token:
/// what it came to, and whether it created the list.
fn log_in<S: Store + Send + 'static>(
    relay: &Arc<Mutex<Relay<S>>>,
    trust: Trust,
    now: u64,
) -> (Result<Token, CourierError>, bool) {
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(move || now);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    let mut login = LoginMachine::start(trust, AUDIENCE);
    drive(&mut login, &mut link, None, &Signal::new());
    let outcome = login.outcome().cloned().expect("the login is over");
    (outcome, login.created())
}

fn ask<S: Store>(
    relay: &mut Relay<S>,
    source: Option<IpAddr>,
    request: &Request,
    now: u64,
) -> Response {
    Response::from_json(&relay.anonymous(source, &request.to_json(), now)).unwrap()
}

/// A challenge for `node` of `list`: its nonce, the instant it expires at, whether the relay holds
/// the list, and the work a creation of it takes.
fn challenge<S: Store>(
    relay: &mut Relay<S>,
    source: Option<IpAddr>,
    list: &str,
    node: u64,
    now: u64,
) -> (String, u64, bool, Option<u32>) {
    match ask(
        relay,
        source,
        &Request::Challenge {
            list: list.to_string(),
            node,
        },
        now,
    ) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            held,
            work,
            ..
        } => (nonce, expires_at_ms, held, work),
        other => panic!("a challenge, and not {other:?}"),
    }
}

/// The creation of `list` by the device of `node`, signed by `signer`, with the entries `log`,
/// against the challenge `nonce` that expires at `expires_at_ms`, with work of `bits`.
fn create_request(
    list: &str,
    node: u64,
    signer: &Identity,
    log: &[Entry],
    (nonce, expires_at_ms): (&str, u64),
    bits: u32,
) -> Request {
    let nonce_bytes = encoding::decode(nonce).unwrap();
    let bytes =
        sce_rust_trust::login::signing_bytes(AUDIENCE, list, node, &nonce_bytes, expires_at_ms);
    let work = creation::find_work(&nonce_bytes, &log[0].hash(), bits).unwrap();
    Request::Create(Box::new(Create {
        login: Login {
            list: list.to_string(),
            node,
            nonce: nonce.to_string(),
            expires_at_ms,
            signature: encoding::encode(&signer.sign(&bytes)),
            entries: log.iter().map(Entry::to_json).collect(),
        },
        work,
    }))
}

/// A creation of the list by alice's device at `now` from `source`, made by hand.
fn create<S: Store>(relay: &mut Relay<S>, source: Option<IpAddr>, now: u64) -> Response {
    let (nonce, expires_at_ms, held, work) = challenge(relay, source, &list(), 1, now);
    assert!(!held);
    assert_eq!(work, Some(BITS));
    let request = create_request(
        &list(),
        1,
        &person(1),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    ask(relay, source, &request, now)
}

fn refused(response: &Response) -> (Code, String) {
    match response {
        Response::Refused { code, reason } => (code.clone(), reason.clone()),
        other => panic!("a refusal, and not {other:?}"),
    }
}

/// The entries of the log `relay` holds of the list, read with an operator's token.
fn held_log<S: Store>(relay: &mut Relay<S>) -> Vec<String> {
    let operator = authority().issue(&list(), 1, u64::MAX);
    match Response::from_json(&relay.handle(
        &operator,
        &Request::Entries { from: 0 }.to_json(),
        NOW,
    ))
    .unwrap()
    {
        Response::Entries { entries, .. } => entries,
        other => panic!("entries, and not {other:?}"),
    }
}

fn at(text: &str) -> Option<IpAddr> {
    Some(text.parse().unwrap())
}

#[test]
fn a_device_whose_list_the_relay_does_not_hold_creates_it_with_its_login() {
    let relay = Arc::new(Mutex::new(relay()));
    let (token, created) = log_in(&relay, device(1), NOW);
    let token = token.expect("the login created the list and came to a token");
    assert!(created);
    assert_eq!(
        authority().verify(&token.token, NOW).unwrap().list,
        list(),
        "a token of the authority for the list"
    );
    {
        let mut held = relay.lock().unwrap();
        assert!(held.is_created_openly(&list()));
        // The log as far as alice's grant, and nothing past it: the rest is her round's to give.
        assert_eq!(
            held_log(&mut *held),
            entries()[..2]
                .iter()
                .map(Entry::to_json)
                .collect::<Vec<_>>()
        );
        // The list is held now: a challenge for it asks no work.
        let (_, _, held_now, work) = challenge(&mut *held, None, &list(), 2, NOW);
        assert!(held_now);
        assert_eq!(work, None);
    }
    // And the next login is a login.
    let (again, created) = log_in(&relay, device(1), NOW);
    again.expect("alice logs in");
    assert!(!created);
}

/// A relay that takes no creation tells a device nothing of work, and its login is refused as one
/// of a list the relay holds no log of.
#[test]
fn a_relay_that_takes_no_creation_asks_no_work_and_creates_nothing() {
    let relay = Arc::new(Mutex::new(relay_over(
        MemoryStore::new(),
        None,
        Limits::default(),
    )));
    let (_, _, held, work) = challenge(&mut *relay.lock().unwrap(), None, &list(), 1, NOW);
    assert!(!held);
    assert_eq!(work, None);
    let (outcome, created) = log_in(&relay, device(1), NOW);
    assert!(
        matches!(
            outcome,
            Err(CourierError::Relay {
                code: Code::Behind,
                ..
            })
        ),
        "{outcome:?}"
    );
    assert!(!created);

    // A creation made by hand is refused as a request this relay does not offer.
    let mut guard = relay.lock().unwrap();
    let closed: &mut Relay<MemoryStore> = &mut guard;
    let (nonce, expires_at_ms, _, _) = challenge(closed, None, &list(), 1, NOW);
    let request = create_request(
        &list(),
        1,
        &person(1),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    let (code, reason) = refused(&ask(closed, None, &request, NOW));
    assert_eq!(code, Code::Invalid);
    assert!(reason.contains("`create`"), "{reason}");
    assert_eq!(closed.records(&list()), 0);
}

#[test]
fn a_creation_with_too_little_work_is_refused() {
    let mut relay = relay();
    let (nonce, expires_at_ms, _, _) = challenge(&mut relay, None, &list(), 1, NOW);
    let nonce_bytes = encoding::decode(&nonce).unwrap();
    let genesis = entries()[0].hash();
    // A counter that falls short of the bits asked: the first one the work does not hold for.
    let short = (0..)
        .find(|counter| !creation::work_holds(&nonce_bytes, &genesis, *counter, BITS))
        .unwrap();
    let Request::Create(mut made) = create_request(
        &list(),
        1,
        &person(1),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    ) else {
        unreachable!()
    };
    made.work = short;
    let (code, reason) = refused(&ask(&mut relay, None, &Request::Create(made), NOW));
    assert_eq!(code, Code::Unauthorized);
    assert!(reason.contains("zero bits"), "{reason}");
    assert_eq!(relay.records(&list()), 0, "nothing was kept");
}

#[test]
fn a_creation_under_a_name_that_does_not_certify_its_genesis_is_refused() {
    let mut relay = relay();
    // The name another administrator's key makes, with this administrator's genesis: one device
    // taking the name another made.
    let stranger = Identity::from_seed([66; 32]);
    let taken = creation::list_name(b"household", &stranger.public()).unwrap();
    let log = log_of(&taken, &admin());
    let (nonce, expires_at_ms, held, work) = challenge(&mut relay, None, &taken, 1, NOW);
    assert!(!held && work == Some(BITS));
    let request = create_request(
        &taken,
        1,
        &person(1),
        &log[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    let (code, reason) = refused(&ask(&mut relay, None, &request, NOW));
    assert_eq!(code, Code::Forbidden);
    assert!(reason.contains("n1."), "{reason}");

    // A name of no such form at all.
    let plain = log_of("home", &admin());
    let (nonce, expires_at_ms, _, _) = challenge(&mut relay, None, "home", 1, NOW);
    let request = create_request(
        "home",
        1,
        &person(1),
        &plain[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    assert_eq!(
        refused(&ask(&mut relay, None, &request, NOW)).0,
        Code::Forbidden
    );
    assert_eq!(relay.records(&taken) + relay.records("home"), 0);
}

#[test]
fn a_creation_signed_by_no_key_its_entries_grant_the_node_is_refused() {
    let mut relay = relay();
    let (nonce, expires_at_ms, _, _) = challenge(&mut relay, None, &list(), 1, NOW);
    // Signed by bob's key for alice's node.
    let request = create_request(
        &list(),
        1,
        &person(2),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    assert_eq!(
        refused(&ask(&mut relay, None, &request, NOW)).0,
        Code::Unauthorized
    );
    // Bob's node, with entries that do not reach his grant.
    let (nonce, expires_at_ms, _, _) = challenge(&mut relay, None, &list(), 2, NOW);
    let request = create_request(
        &list(),
        2,
        &person(2),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    assert_eq!(
        refused(&ask(&mut relay, None, &request, NOW)).0,
        Code::Forbidden
    );
    assert_eq!(relay.records(&list()), 0, "nothing was kept");
    assert!(!relay.is_created_openly(&list()));
}

#[test]
fn a_relay_that_holds_as_many_lists_as_it_keeps_takes_no_creation() {
    let mut relay = relay_over(
        MemoryStore::new(),
        Some(creation_policy()),
        Limits {
            max_lists: 1,
            ..Limits::default()
        },
    );
    // The one list the relay keeps, given by an operator's token.
    let operator = authority().issue("home", 1, u64::MAX);
    let other = log_of("home", &admin());
    let answer = relay.handle(
        &operator,
        &Request::Append {
            entry: other[0].to_json(),
        }
        .to_json(),
        NOW,
    );
    assert!(answer.contains("\"appended\""), "{answer}");
    let (code, _) = refused(&create(&mut relay, None, NOW));
    assert_eq!(code, Code::TooLarge);
    assert_eq!(relay.records(&list()), 0);
}

#[test]
fn a_source_and_the_relay_are_held_to_their_creations() {
    let tight = CreationPolicy {
        creations_per_minute: 1,
        creations_per_hour: 2,
        ..creation_policy()
    };
    let mut relay = relay_over(MemoryStore::new(), Some(tight), Limits::default());
    let (a, b, c) = (at("198.51.100.1"), at("198.51.100.2"), at("198.51.100.3"));
    assert!(matches!(create(&mut relay, a, NOW), Response::Token { .. }));
    // The same source again within the minute, for another list: refused before anything else.
    let second = creation::list_name(b"second", &admin().public()).unwrap();
    let log = log_of(&second, &admin());
    let by = |relay: &mut Relay<MemoryStore>, source, now| {
        let (nonce, expires_at_ms, _, _) = challenge(relay, source, &second, 1, now);
        let request = create_request(
            &second,
            1,
            &person(1),
            &log[..2],
            (&nonce, expires_at_ms),
            BITS,
        );
        ask(relay, source, &request, now)
    };
    let (code, _) = refused(&by(&mut relay, a, NOW));
    assert!(matches!(code, Code::RateLimited { .. }), "{code:?}");
    // Another source makes the relay's second creation of the hour.
    assert!(matches!(by(&mut relay, b, NOW), Response::Token { .. }));
    // A third source, the next minute: the relay has taken as many as it takes this hour.
    let third = creation::list_name(b"third", &admin().public()).unwrap();
    let log = log_of(&third, &admin());
    let (nonce, expires_at_ms, _, _) = challenge(&mut relay, c, &third, 1, NOW + 60_000);
    let request = create_request(
        &third,
        1,
        &person(1),
        &log[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    let (code, reason) = refused(&ask(&mut relay, c, &request, NOW + 60_000));
    assert!(matches!(code, Code::RateLimited { .. }), "{code:?}");
    assert!(reason.contains("this hour"), "{reason}");
    assert_eq!(relay.records(&third), 0);
}

#[test]
fn a_lapse_removes_the_lists_created_openly_past_their_days_and_no_other() {
    let mut relay = relay();
    assert!(matches!(
        create(&mut relay, None, NOW),
        Response::Token { .. }
    ));
    // A list created with a token of the authority.
    let operator = authority().issue("home", 1, u64::MAX);
    for entry in log_of("home", &admin()) {
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
    assert!(!relay.is_created_openly("home"));

    let due = NOW + IDLE_MS + TOUCH_STEP_MS;
    let early = relay.lapse(IDLE_MS, due - 1);
    assert_eq!((early.removed, early.kept), (0, 1), "not yet");
    let lapsed = relay.lapse(IDLE_MS, due);
    assert_eq!((lapsed.removed, lapsed.kept), (1, 0));
    assert!(lapsed.failures.is_empty());
    assert_eq!(relay.records(&list()), 0, "the list is gone whole");
    assert_eq!(
        relay.records("home"),
        4,
        "a list created with a token stays"
    );
    let far = relay.lapse(IDLE_MS, due + 10 * IDLE_MS);
    assert_eq!(far.removed, 0);
    assert_eq!(relay.records("home"), 4);

    // The name is free for the list's own genesis again: a challenge asks the work of it.
    let (_, _, held, work) = challenge(&mut relay, None, &list(), 1, due);
    assert!(!held);
    assert_eq!(work, Some(BITS));
}

#[test]
fn a_login_keeps_a_list_alive_across_a_restart() {
    let dir = scratch("touched");
    let store = || FileStore::open(&dir).unwrap();
    let relay = Arc::new(Mutex::new(relay_over(
        store(),
        Some(creation_policy()),
        Limits::default(),
    )));
    let (created, _) = log_in(&relay, device(1), NOW);
    created.unwrap();
    // Bob logs in ten days later, bringing his grant: the list is touched, and its mark written.
    let later = NOW + 10 * DAY_MS;
    let (token, created) = log_in(&relay, device(2), later);
    token.expect("bob logs in");
    assert!(!created, "a login of a list the relay holds");
    let due = NOW + IDLE_MS + TOUCH_STEP_MS;
    assert_eq!(relay.lock().unwrap().lapse(IDLE_MS, due).removed, 0);
    drop(relay);

    // The relay starts again from its store, which holds the mark of bob's login.
    let mut again = relay_over(store(), Some(creation_policy()), Limits::default());
    assert!(again.is_created_openly(&list()));
    assert_eq!(again.lapse(IDLE_MS, due).removed, 0, "bob's login kept it");
    assert_eq!(again.records(&list()), 3);
    assert_eq!(
        again
            .lapse(IDLE_MS, later + IDLE_MS + TOUCH_STEP_MS)
            .removed,
        1
    );
    drop(again);
    // The removal was the store's too: a relay read again holds nothing of the list.
    let mut third = relay_over(store(), Some(creation_policy()), Limits::default());
    assert_eq!(third.records(&list()), 0);
    assert!(!third.is_created_openly(&list()));
    assert_eq!(third.lapse(IDLE_MS, u64::MAX / 2).kept, 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_device_that_holds_a_list_that_lapsed_creates_it_again_and_every_device_reads_it_anew() {
    let relay = Arc::new(Mutex::new(relay()));
    let (alice, bob) = (device(1), device(2));
    let write = |trust: &Trust, id: u32, node: u64| {
        let data = format!(
            "{{\"by\":\"{node}\",\"wallTime\":\"{}\",\"counter\":0,\"ops\":[\
             {{\"kind\":0,\"id\":{id},\"field\":3,\"value\":\"1\",\"text\":\"\",\
             \"wallTime\":\"0\",\"counter\":0,\"nodeId\":\"0\"}}]}}",
            1000 * id
        );
        assert!(matches!(
            keep(trust, "record_list.change", &data),
            Kept::Taken
        ));
    };

    // Alice's device creates the list and writes; bob's logs in and reads it.
    let token_a = log_in(&relay, alice.clone(), NOW).0.unwrap().token;
    write(&alice, 1, 1);
    let (round, alice_state) = round_from(&relay, CourierState::default(), &alice, &token_a, NOW);
    round.unwrap();
    let token_b = log_in(&relay, bob.clone(), NOW).0.unwrap().token;
    let (round, bob_state) = round_from(&relay, CourierState::default(), &bob, &token_b, NOW);
    round.unwrap();
    assert_eq!(bob.last_seq(1), 1);
    // Alice writes again, which bob does not read before the list lapses.
    write(&alice, 2, 1);
    let (round, alice_state) = round_from(&relay, alice_state, &alice, &token_a, NOW);
    round.unwrap();
    assert_eq!(relay.lock().unwrap().records(&list()), 6);

    let due = NOW + IDLE_MS + TOUCH_STEP_MS;
    assert_eq!(relay.lock().unwrap().lapse(IDLE_MS, due).removed, 1);
    assert_eq!(relay.lock().unwrap().records(&list()), 0);

    // Bob's device still holds the list: its login creates it again, with the same genesis and
    // name, as far as his own grant.
    let (token, created) = log_in(&relay, bob.clone(), due);
    let token_b = token.expect("bob's login created the list again").token;
    assert!(created);
    assert!(relay.lock().unwrap().is_created_openly(&list()));
    assert_eq!(relay.lock().unwrap().records(&list()), 3);
    // He writes, and his round gives the relay what it lacks of what he holds: carol's grant,
    // alice's first write and his own.
    write(&bob, 3, 2);
    let (round, _) = round_from(&relay, bob_state, &bob, &token_b, due);
    let round = round.unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);
    assert_eq!(round.pushed, 3);
    assert_eq!(relay.lock().unwrap().records(&list()), 6);

    // Alice's state read six records of the history before; the relay's heads say its history
    // began again, so her round reads it from its start, takes bob's write, which a read from her
    // old place would have passed, and gives back her second write.
    let (token, created) = log_in(&relay, alice.clone(), due);
    let token_a = token.expect("alice logs in").token;
    assert!(!created);
    let (round, _) = round_from(&relay, alice_state, &alice, &token_a, due);
    let round = round.unwrap();
    assert!(round.refused.is_empty(), "{:?}", round.refused);
    assert_eq!(alice.last_seq(2), 1, "bob's write was read");
    assert_eq!(round.pushed, 1, "her second write was given back");
    assert_eq!(relay.lock().unwrap().records(&list()), 7);
    assert_eq!(
        held_log(&mut *relay.lock().unwrap()),
        entries().iter().map(Entry::to_json).collect::<Vec<_>>()
    );
}

/// A round of the device `trust` at `now` with `token`, from the state `state` it kept, as a host
/// that keeps its courier's state between rounds makes it: what it came to, and the state it keeps.
fn round_from(
    relay: &Arc<Mutex<Relay<MemoryStore>>>,
    state: CourierState,
    trust: &Trust,
    token: &str,
    now: u64,
) -> (Result<Round, CourierError>, CourierState) {
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(move || now);
    let mut link = DirectLink::new(relay.clone(), token.to_string(), clock);
    let mut machine = RoundMachine::start(RoundKind::Sync, state, trust.clone());
    drive(&mut machine, &mut link, None, &Signal::new());
    let outcome = match machine.poll() {
        Step::Done(outcome) => outcome,
        Step::Request(request) => panic!("the round still asks {request:?}"),
    };
    (outcome, machine.into_state())
}

#[test]
fn a_store_read_again_holds_the_mark_of_a_list_created_openly_and_drops_one_with_no_list() {
    let dir = scratch("marks");
    let mut store = FileStore::open(&dir).unwrap();
    // What a crash between a creation's mark and its first record leaves.
    store
        .mark(
            "lost",
            &Openly {
                created_at_ms: NOW,
                touched_at_ms: NOW,
            },
        )
        .unwrap();
    let mut relay = relay_over(store, Some(creation_policy()), Limits::default());
    assert!(matches!(
        create(&mut relay, None, NOW),
        Response::Token { .. }
    ));
    drop(relay);
    let mut reopened = FileStore::open(&dir).unwrap();
    let marks = reopened.marks().unwrap();
    assert_eq!(
        marks
            .iter()
            .map(|(list, _)| list.clone())
            .collect::<Vec<_>>(),
        vec![list()],
        "the mark with no list was removed as the relay started"
    );
    let again = relay_over(reopened, Some(creation_policy()), Limits::default());
    assert!(again.is_created_openly(&list()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_relay_of_several_slots_creates_and_lapses_as_a_relay_of_one_does() {
    let shards = Shards::open(
        4,
        |_| Ok(MemoryStore::new()),
        authority(),
        Limits::default(),
    )
    .unwrap()
    .with_login(doorway(Some(creation_policy())));
    let answer = |request: &Request| {
        Response::from_json(&shards.handle_anonymous(None, &request.to_json(), NOW)).unwrap()
    };
    let (nonce, expires_at_ms) = match answer(&Request::Challenge {
        list: list(),
        node: 1,
    }) {
        Response::Challenge {
            nonce,
            expires_at_ms,
            held: false,
            work: Some(BITS),
            ..
        } => (nonce, expires_at_ms),
        other => panic!("{other:?}"),
    };
    let request = create_request(
        &list(),
        1,
        &person(1),
        &entries()[..2],
        (&nonce, expires_at_ms),
        BITS,
    );
    assert!(matches!(answer(&request), Response::Token { .. }));
    let lapsed = shards.lapse(IDLE_MS, NOW + IDLE_MS + TOUCH_STEP_MS);
    assert_eq!((lapsed.removed, lapsed.kept), (1, 0));
    assert!(matches!(
        answer(&Request::Challenge {
            list: list(),
            node: 1,
        }),
        Response::Challenge { held: false, .. }
    ));
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sce-relay-lapse-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}
