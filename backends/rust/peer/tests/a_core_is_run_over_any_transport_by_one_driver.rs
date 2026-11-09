// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The peer core run over a transport by the one driver (`sce_rust_relay::drive`), with a relay in
// the same process and no network. What is asked: that the driver puts the requests a host carrying
// the texts by hand puts, in the same order, and the core comes to the same outcomes, over one
// history of the list in two universes that never meet; that a transport that fails in the middle of
// a round ends the round as a failure of the link and leaves the next round to hand over what the
// broken one took in, and nothing twice; that a round whose transport waits ends when it is
// cancelled from another thread, and when its deadline passes, long before it would otherwise; that
// a cancellation raised before the drive carries nothing; and that a drive with no exchange running
// asks nothing.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use sce_rust_peer::{
    drive, CourierError, Driven, Kept, Outcome, Peer, PeerError, Profile, RoundKind, Signal,
    Transport, TransportError,
};
use sce_rust_relay::logs::{capture, list_digest};
use sce_rust_relay::{
    Authority, Code, DirectLink, Doorway, Limits, LoginPolicy, MemoryStore, Notice, Relay,
};
use sce_rust_trust::membership::{Body, Entry, Log};
use sce_rust_trust::Identity;
use serde_json::json;

const LIST: &str = "home";
const ALICE: u64 = 1;
const BOB: u64 = 2;
const CAROL: u64 = 3;

const CHANGE: &str = "record_list.change";

fn person(node: u64) -> Identity {
    Identity::from_seed([node as u8; 32])
}

fn administrator() -> Identity {
    Identity::from_seed([9; 32])
}

fn genesis() -> Entry {
    Log::genesis_draft(
        LIST,
        Body::Genesis {
            administrators: vec![administrator().public()],
            quorum: 1,
            profile: Profile::Untrusted,
        },
    )
    .sign(&administrator())
    .finish()
}

/// A change of one write of `value` to field 3 of row `id`, stamped by `node` at `wall`.
fn write(id: u32, node: u64, wall: i64, value: i64) -> String {
    json!({"by": node.to_string(), "wallTime": wall.to_string(), "counter": 0, "ops": [
        {"kind": 0, "id": id, "field": 3, "value": value.to_string(), "text": "",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string()
}

/// The change of `by`'s stamped at `at` that writes over the write of row `id` that `node`
/// stamped at `wall`.
fn supersede(id: u32, node: u64, wall: i64, by: u64, at: i64) -> String {
    json!({"by": by.to_string(), "wallTime": at.to_string(), "counter": 0, "ops": [
        {"kind": 1, "id": id, "field": 3, "value": "0", "text": "",
         "wallTime": wall.to_string(), "counter": 0, "nodeId": node.to_string()}]})
    .to_string()
}

/// Alice's device, which holds the administrator's key: it opens the list and grants bob writing
/// and carol reading.
fn alice() -> Peer {
    let mut alice = Peer::open(LIST);
    alice.set_identity(ALICE, person(ALICE));
    alice.set_administrator(administrator());
    assert_eq!(
        alice.open_list(Profile::Untrusted).unwrap(),
        genesis().hash()
    );
    for (node, role) in [(BOB, 2u8), (CAROL, 1)] {
        alice.grant(node, role, person(node).public()).unwrap();
    }
    alice
}

/// A device invited with the hash of the genesis.
fn invited(node: u64) -> Peer {
    let mut peer = Peer::invited(LIST, genesis().hash());
    peer.set_identity(node, person(node));
    peer
}

type Shared = Arc<Mutex<Relay<MemoryStore>>>;

fn authority() -> Authority {
    Authority::new([3; 32], "the-relay")
}

fn relay(limits: Limits) -> Shared {
    Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority(), limits).unwrap(),
    ))
}

/// The transport of the device of `node` to `relay` in this process.
fn direct(relay: &Shared, node: u64) -> DirectLink<MemoryStore> {
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    DirectLink::new(
        relay.clone(),
        authority().issue(LIST, node, u64::MAX),
        clock,
    )
}

type Carried = Arc<Mutex<Vec<String>>>;

/// A transport over another that keeps every request text it carried, and that fails the request
/// whose number (from 1) is `fail_at`, as a connection that closes does.
struct Recording<T> {
    inner: T,
    carried: Carried,
    fail_at: Option<usize>,
}

impl<T: Transport> Recording<T> {
    fn over(inner: T) -> (Self, Carried) {
        let carried = Carried::default();
        let recording = Self {
            inner,
            carried: carried.clone(),
            fail_at: None,
        };
        (recording, carried)
    }
}

impl<T: Transport> Transport for Recording<T> {
    fn exchange(
        &mut self,
        request: &str,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError> {
        let mut carried = self.carried.lock().unwrap();
        carried.push(request.to_string());
        if self.fail_at == Some(carried.len()) {
            return Err(TransportError::Link("the connection closed".to_string()));
        }
        drop(carried);
        self.inner.exchange(request, deadline, cancel)
    }

    fn take_notices(&mut self) -> Vec<Notice> {
        self.inner.take_notices()
    }

    fn network(&self) -> Signal {
        self.inner.network()
    }
}

/// A round of `kind` of `peer`, run by the driver over `transport` with no deadline, and what it
/// came to.
fn run(peer: &mut Peer, transport: &mut impl Transport, kind: RoundKind) -> Outcome {
    peer.start(kind).unwrap();
    assert_eq!(drive(peer, transport, None, &Signal::new()), Driven::Ended);
    peer.take_outcome()
        .expect("the exchange ended")
        .expect("the exchange held")
}

/// A round of `kind` of `peer`, the device of `node`, with every text carried to `relay` by hand, as
/// a host that carries the texts itself does: what it came to, and the requests it asked.
fn by_hand(peer: &mut Peer, relay: &Shared, node: u64, kind: RoundKind) -> (Outcome, Vec<String>) {
    let token = authority().issue(LIST, node, u64::MAX);
    let mut asked = Vec::new();
    peer.start(kind).unwrap();
    while let Some(request) = peer.poll_request() {
        let response = relay.lock().unwrap().handle(&token, &request, 0);
        asked.push(request);
        peer.respond(&response).unwrap();
    }
    let outcome = peer
        .take_outcome()
        .expect("the exchange ended")
        .expect("the exchange held");
    (outcome, asked)
}

fn taken(carried: &Carried) -> Vec<String> {
    std::mem::take(&mut *carried.lock().unwrap())
}

#[test]
fn the_driver_asks_and_comes_to_what_a_host_carrying_the_texts_by_hand_does() {
    let (hand_relay, driven_relay) = (relay(Limits::default()), relay(Limits::default()));
    let (mut hand_alice, mut driven_alice) = (alice(), alice());
    let (mut hand_bob, mut driven_bob) = (invited(BOB), invited(BOB));
    let (mut alice_link, alice_carried) = Recording::over(direct(&driven_relay, ALICE));
    let (mut bob_link, bob_carried) = Recording::over(direct(&driven_relay, BOB));

    for alice in [&mut hand_alice, &mut driven_alice] {
        assert_eq!(
            alice.outbound(CHANGE, &write(1, ALICE, 1000, 10)),
            Kept::Taken
        );
        assert_eq!(
            alice.outbound(CHANGE, &write(1, ALICE, 2000, 20)),
            Kept::Taken
        );
    }
    let (expected, asked) = by_hand(&mut hand_alice, &hand_relay, ALICE, RoundKind::Sync);
    let outcome = run(&mut driven_alice, &mut alice_link, RoundKind::Sync);
    assert_eq!(outcome, expected);
    assert_eq!(taken(&alice_carried), asked);
    assert_eq!(outcome.round.pushed, 4 + 2, "the log and the two writes");

    let (expected, asked) = by_hand(&mut hand_bob, &hand_relay, BOB, RoundKind::Sync);
    let outcome = run(&mut driven_bob, &mut bob_link, RoundKind::Sync);
    assert_eq!(outcome, expected);
    assert_eq!(taken(&bob_carried), asked);
    assert_eq!(
        outcome
            .events()
            .iter()
            .filter(|(event, _)| event == CHANGE)
            .count(),
        2
    );

    for alice in [&mut hand_alice, &mut driven_alice] {
        assert_eq!(
            alice.outbound(CHANGE, &supersede(1, ALICE, 1000, ALICE, 2500)),
            Kept::Taken
        );
    }
    let (expected, asked) = by_hand(&mut hand_alice, &hand_relay, ALICE, RoundKind::Sync);
    assert_eq!(
        run(&mut driven_alice, &mut alice_link, RoundKind::Sync),
        expected
    );
    assert_eq!(taken(&alice_carried), asked);
    let (expected, asked) = by_hand(&mut hand_bob, &hand_relay, BOB, RoundKind::Resync);
    assert_eq!(
        run(&mut driven_bob, &mut bob_link, RoundKind::Resync),
        expected
    );
    assert_eq!(taken(&bob_carried), asked);
    assert_eq!(driven_bob.trust().heads(), hand_bob.trust().heads());
}

/// A relay that holds alice's log and four writes of hers, and gives two records a batch, so that a
/// round of bob's has several steps to stop between.
fn four_writes() -> Shared {
    let relay = relay(Limits {
        max_feed: 2,
        ..Limits::default()
    });
    let mut alice = alice();
    for n in 1..=4 {
        assert_eq!(
            alice.outbound(CHANGE, &write(n, ALICE, 1000 + i64::from(n), i64::from(n))),
            Kept::Taken
        );
    }
    let outcome = run(&mut alice, &mut direct(&relay, ALICE), RoundKind::Sync);
    assert!(outcome.refused().is_empty(), "{:?}", outcome.refused());
    relay
}

#[test]
fn a_transport_that_fails_mid_round_leaves_the_next_round_to_hand_over_what_was_taken() {
    let unbroken = four_writes();
    let reference = run(
        &mut invited(BOB),
        &mut direct(&unbroken, BOB),
        RoundKind::Sync,
    );
    assert_eq!(
        reference
            .events()
            .iter()
            .filter(|(event, _)| event == CHANGE)
            .count(),
        4
    );

    let broken = four_writes();
    let mut bob = invited(BOB);
    let (mut link, carried) = Recording::over(direct(&broken, BOB));
    // The log is records 0 to 3 and alice's writes 4 to 7: the fourth batch is the last two writes.
    let feeds_before_the_fourth = {
        let mut probe = invited(BOB);
        let (mut probe_link, probe_carried) = Recording::over(direct(&broken, BOB));
        run(&mut probe, &mut probe_link, RoundKind::Sync);
        let asked = taken(&probe_carried);
        let fourth_feed = asked
            .iter()
            .enumerate()
            .filter(|(_, request)| request.contains("\"op\":\"feed\""))
            .nth(3)
            .map(|(at, _)| at)
            .expect("a round asks for four batches");
        fourth_feed + 1
    };
    link.fail_at = Some(feeds_before_the_fourth);

    bob.start(RoundKind::Sync).unwrap();
    assert_eq!(
        drive(&mut bob, &mut link, None, &Signal::new()),
        Driven::Ended,
        "a failure of the link is the end of the exchange, not an interruption of the drive"
    );
    assert_eq!(
        bob.take_outcome(),
        Some(Err(CourierError::Link("the connection closed".to_string())))
    );
    assert_eq!(taken(&carried).len(), feeds_before_the_fourth);

    link.fail_at = None;
    let resumed = run(&mut bob, &mut link, RoundKind::Sync);
    assert_eq!(
        resumed.events(),
        reference.events(),
        "the roster, then the writes the broken round took in, then the rest, each once"
    );
    let again = run(&mut bob, &mut link, RoundKind::Sync);
    assert!(again.events().is_empty(), "{:?}", again.events());
}

/// A transport whose relay never answers: an exchange waits until its cancellation is raised or its
/// deadline passes, and says which, as the contract asks of every transport.
struct NeverAnswers;

impl Transport for NeverAnswers {
    fn exchange(
        &mut self,
        _request: &str,
        deadline: Option<Instant>,
        cancel: &Signal,
    ) -> Result<String, TransportError> {
        let woken = Arc::new((Mutex::new(false), Condvar::new()));
        let _watch = {
            let woken = woken.clone();
            cancel.watch(move || {
                *woken.0.lock().unwrap() = true;
                woken.1.notify_all();
            })
        };
        let mut raised = woken.0.lock().unwrap();
        loop {
            if *raised || cancel.is_raised() {
                return Err(TransportError::Cancelled);
            }
            match deadline {
                Some(deadline) => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err(TransportError::Deadline);
                    }
                    raised = woken.1.wait_timeout(raised, deadline - now).unwrap().0;
                }
                None => raised = woken.1.wait(raised).unwrap(),
            }
        }
    }

    fn take_notices(&mut self) -> Vec<Notice> {
        Vec::new()
    }

    fn network(&self) -> Signal {
        Signal::new()
    }
}

#[test]
fn a_round_whose_transport_waits_ends_when_cancelled_and_when_its_deadline_passes() {
    let relay = relay(Limits::default());
    let mut alice = alice();
    assert_eq!(
        alice.outbound(CHANGE, &write(1, ALICE, 1000, 10)),
        Kept::Taken
    );

    // Cancelled from another thread, a minute before its deadline.
    alice.start(RoundKind::Sync).unwrap();
    let cancel = Signal::new();
    let raiser = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            cancel.raise();
        })
    };
    let began = Instant::now();
    let driven = drive(
        &mut alice,
        &mut NeverAnswers,
        Some(began + Duration::from_secs(60)),
        &cancel,
    );
    raiser.join().unwrap();
    assert_eq!(driven, Driven::Cancelled);
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "{:?}",
        began.elapsed()
    );
    assert_eq!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(
            TransportError::Cancelled.to_string()
        )))
    );

    // Its deadline passes.
    alice.start(RoundKind::Sync).unwrap();
    let began = Instant::now();
    let driven = drive(
        &mut alice,
        &mut NeverAnswers,
        Some(began + Duration::from_millis(300)),
        &Signal::new(),
    );
    assert_eq!(driven, Driven::DeadlinePassed);
    let waited = began.elapsed();
    assert!(
        waited >= Duration::from_millis(300) && waited < Duration::from_secs(20),
        "{waited:?}"
    );
    assert_eq!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(
            TransportError::Deadline.to_string()
        )))
    );

    // A cancellation raised before the drive: the request is not carried.
    let (mut link, carried) = Recording::over(direct(&relay, ALICE));
    alice.start(RoundKind::Sync).unwrap();
    assert_eq!(
        drive(&mut alice, &mut link, None, &cancel),
        Driven::Cancelled
    );
    assert!(taken(&carried).is_empty(), "nothing was carried");
    assert!(matches!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(_)))
    ));

    // Nothing was lost by the rounds that did not end: the next one gives the relay all of it.
    let outcome = run(&mut alice, &mut link, RoundKind::Sync);
    assert_eq!(outcome.round.pushed, 4 + 1, "the log and the write");
    assert!(
        !taken(&carried).is_empty(),
        "the round carried its requests"
    );

    // With no exchange running, a drive asks nothing.
    assert_eq!(
        drive(&mut alice, &mut link, None, &Signal::new()),
        Driven::Ended
    );
    assert!(taken(&carried).is_empty());
    assert_eq!(alice.take_outcome(), None);
}

/// A device logs in through its core (ADR 0080): over a transport that presents no token, the core
/// asks for a challenge and logs in with its person's key, the outcome holds the token, and the
/// transport given it carries the device's rounds. A round with no token is the relay's refusal of
/// the token, which is when a host logs in; a relay that names itself otherwise than the device was
/// configured for is signed nothing.
#[test]
fn a_core_logs_in_over_a_transport_with_no_token_and_runs_its_rounds_with_the_token() {
    let relay: Shared = Arc::new(Mutex::new(
        Relay::open(MemoryStore::new(), authority(), Limits::default())
            .unwrap()
            .with_login(Doorway::with_key(LoginPolicy::default(), [6; 32])),
    ));
    let none = Signal::new();
    // The list is created with a token of the authority, as a relay whose creation is closed takes.
    let mut alice = alice();
    let mut operator = direct(&relay, ALICE);
    alice.start(RoundKind::Sync).unwrap();
    drive(&mut alice, &mut operator, None, &none);
    assert!(matches!(alice.take_outcome(), Some(Ok(_))));

    assert_eq!(
        Peer::invited(LIST, genesis().hash()).login("the-relay"),
        Err(PeerError::NoPerson),
        "a device with no person logs in as no one"
    );
    let mut bob = invited(BOB);
    assert_eq!(bob.login("the-relay"), Ok(()));
    let clock: Arc<dyn Fn() -> u64 + Send + Sync> = Arc::new(|| 0);
    let mut link = DirectLink::new(relay.clone(), String::new(), clock);
    // The login begun above is abandoned by the round, which the relay refuses for want of a token.
    bob.start(RoundKind::Sync).unwrap();
    drive(&mut bob, &mut link, None, &none);
    match bob.take_outcome() {
        Some(Err(CourierError::Relay {
            code: Code::Unauthorized,
            ..
        })) => {}
        other => panic!("the relay's refusal of the token, and not {other:?}"),
    }

    bob.login("another-relay").unwrap();
    drive(&mut bob, &mut link, None, &none);
    match bob.take_outcome() {
        Some(Err(CourierError::Unexpected(why))) => assert!(why.contains("another-relay"), "{why}"),
        other => panic!("a device configured for another relay stops, and not {other:?}"),
    }

    bob.login("the-relay").unwrap();
    assert_eq!(drive(&mut bob, &mut link, None, &none), Driven::Ended);
    let logged_in: Outcome = bob.take_outcome().unwrap().unwrap();
    assert!(
        logged_in.events().is_empty(),
        "a login tells the machine nothing"
    );
    let token = logged_in.token.expect("a login's outcome holds its token");
    assert_eq!(
        token.expires_at_ms, 3_600_000,
        "an hour by the relay's clock"
    );
    assert_eq!(
        authority()
            .verify(&token.token, 0)
            .map(|session| session.node),
        Ok(BOB)
    );

    assert!(link.set_token(&token.token));
    bob.start(RoundKind::Sync).unwrap();
    drive(&mut bob, &mut link, None, &none);
    let read = bob.take_outcome().unwrap().unwrap();
    assert_eq!(read.token, None, "a round earns no token");
    assert!(!read.events().is_empty(), "the roster: {read:?}");
}

/// What the core decides is one record each, in counts: a round that ended (the events, what the
/// relay was given, the refusals, what it is still owed), and a round whose link failed (the
/// link's words), beside the driver's record of the failure. The list is named by its digest, and
/// no record holds the list's name or the device's token.
#[test]
fn a_round_and_a_failure_of_the_core_are_each_one_record_with_no_secret() {
    capture::start();
    let shared = relay(Limits::default());
    let mut alice = alice();
    let outcome = run(&mut alice, &mut direct(&shared, ALICE), RoundKind::Sync);
    let digest = list_digest(LIST);
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    let ended = format!(
        "event=exchange_ended list={digest} exchange=round events={} pushed={} taken=0 held=0 \
         dropped=0 refused=0 stalled=false rebuild=false owed=0 earned_token=false",
        outcome.events().len(),
        outcome.round.pushed
    );
    assert!(lines.contains(&ended), "{ended} in {lines:#?}");
    let mut every = lines;

    let (mut failing, _) = Recording::over(direct(&shared, ALICE));
    failing.fail_at = Some(1);
    alice.start(RoundKind::Sync).unwrap();
    assert_eq!(
        drive(&mut alice, &mut failing, None, &Signal::new()),
        Driven::Ended
    );
    assert!(matches!(
        alice.take_outcome(),
        Some(Err(CourierError::Link(_)))
    ));
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    assert!(
        lines.contains(&format!(
            "event=exchange_failed list={digest} exchange=round failure=link \
             reason=\"the connection closed\""
        )),
        "{lines:#?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("event=link_failed op=")
                && line.ends_with("reason=\"the connection closed\" limited=0")),
        "the driver's record: {lines:#?}"
    );
    every.extend(lines);

    let token = authority().issue(LIST, ALICE, u64::MAX);
    for line in &every {
        assert!(!line.contains(LIST), "the list's name: {line}");
        assert!(!line.contains(&token), "the device's token: {line}");
    }
}

/// What became of each send of the machine is one record at `debug`, by its event and its decision,
/// with why when it waits or is refused; and each signing of what waited is one record for each send
/// it decided and for what still waits, and one of the counts at `info`. A device restored from its
/// state keeps a new write waiting: the round's first signing, once the logs are level, finds it
/// still waiting for her chain, and the one after her chain is read back signs it. No record holds
/// the list's name or what a write carries.
#[test]
fn what_becomes_of_each_send_is_logged_by_its_decision_and_never_by_its_content() {
    capture::start();
    let shared = relay(Limits::default());
    let digest = list_digest(LIST);
    let mut alice = alice();
    let milk = json!({"by": ALICE.to_string(), "wallTime": "1000", "counter": 0, "ops": [
        {"kind": 0, "id": 1, "field": 1, "value": "0", "text": "buy milk",
         "wallTime": "0", "counter": 0, "nodeId": "0"}]})
    .to_string();
    assert_eq!(alice.outbound(CHANGE, &milk), Kept::Taken);
    assert_eq!(alice.outbound(CHANGE, &milk), Kept::Held);
    let bread = milk.replace("buy milk", "buy bread");
    assert!(matches!(alice.outbound(CHANGE, &bread), Kept::Refused(_)));
    assert_eq!(
        alice.outbound("record_list.edit", r#"{"id":1}"#),
        Kept::NotSigned
    );
    run(&mut alice, &mut direct(&shared, ALICE), RoundKind::Sync);
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    let kept = |decision: &str| {
        format!("event=send_kept list={digest} send=record_list.change decision={decision}")
    };
    assert!(lines.contains(&kept("taken")), "{lines:#?}");
    assert!(lines.contains(&kept("held")), "{lines:#?}");
    let refused = format!(
        "{} reason=\"record_list.change: this device signed another content",
        kept("refused")
    );
    assert!(
        lines.iter().any(|line| line.starts_with(&refused)),
        "{refused} in {lines:#?}"
    );
    assert!(
        lines.contains(&format!(
            "event=send_kept list={digest} send=record_list.edit decision=not_signed"
        )),
        "{lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("event=waiting_signed")),
        "nothing waited, and nothing is said of it: {lines:#?}"
    );
    let mut every = lines;

    let sealed = alice.save_sealed().unwrap();
    let mut restored = Peer::open(LIST);
    restored
        .restore_sealed(ALICE, &sealed.state, &sealed.key)
        .unwrap();
    let later = write(2, ALICE, 2000, 2);
    assert!(matches!(
        restored.outbound(CHANGE, &later),
        Kept::Waiting(_)
    ));
    let outcome = run(&mut restored, &mut direct(&shared, ALICE), RoundKind::Sync);
    assert_eq!(outcome.taken(), vec![(CHANGE.to_string(), later)]);
    let lines: Vec<String> = capture::taken()
        .into_iter()
        .map(|taken| taken.line)
        .collect();
    let why =
        "\"record_list.change: this device's state came back from a store that may be behind \
               what it signed, and it signs nothing new until a round has read its own chain back \
               from the relay\"";
    let expected = [
        format!("{} reason={why}", kept("waiting")),
        format!("event=send_waited list={digest} decision=waiting count=1 reason={why}"),
        format!("event=waiting_signed list={digest} taken=0 held=0 dropped=0 waiting=1"),
        format!("event=send_waited list={digest} send=record_list.change decision=taken"),
        format!("event=waiting_signed list={digest} taken=1 held=0 dropped=0 waiting=0"),
    ];
    for line in &expected {
        assert!(lines.contains(line), "{line} in {lines:#?}");
    }
    every.extend(lines);

    for line in &every {
        assert!(!line.contains(LIST), "the list's name: {line}");
        assert!(
            !line.contains("milk") && !line.contains("bread"),
            "what a write carries: {line}"
        );
    }
}
