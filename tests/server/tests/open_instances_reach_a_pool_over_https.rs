// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The instances of an open machine reach a keyed pool over the HTTPS binding (docs/adr/0113,
//! step 5; docs/formal/OpenInstances.tla): instances no deployment lists each join, are offered
//! an incarnation, confirm it with their first request and are answered; a response lost on its
//! way is asked for again and answered from the request's completion record, so the request takes
//! effect once; and an instance whose lease ran out is fenced `412`, takes nothing in, and is
//! offered a new incarnation when it joins again.
//!
//! The server is the binding's server (`HttpsServer`) keeping the members in the pool's own
//! store (`HttpsServer::keep_members`), in front of the ledger served as a keyed pool with
//! members (`Pool::with_members`); its host hands the pool each envelope an instance's exchange
//! carried with the judgement of the edge (`Pool::receive_member`) and runs the session it names
//! before the exchange is answered. The client is the test, which writes each exchange as an
//! instance's dialing side does: a join with its nonce, then each request under `If-Match` with
//! its low-water mark.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use sce_forge_runtime::codec::Bounded;
use sce_rust_mesh::endpoint::EngineCall;
use sce_rust_mesh::generated::envelope::Envelope;
use sce_rust_mesh::generated::pattern_kind::PatternKind;
use sce_rust_mesh::generated::payload_codec::PayloadCodec;
use sce_rust_mesh::https::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};
use sce_rust_mesh::https::principal::{Challenge, Login, Offering};
use sce_rust_mesh::https::principal_conformance::TestProof;
use sce_rust_mesh::https::server::{
    Authenticator, Durability, HttpsServer, Judgement, MailboxTransport, ServerHost, ServerSettings,
};
use sce_rust_mesh::https::server::{KeyPins, MemoryCredentials};
use sce_rust_mesh::https::signature::{
    content_digest, normalize_target_uri, RequestSigner, CONTENT_DIGEST_HEADER, NONCE_HEADER,
};
use sce_rust_mesh::https::{
    entity_tag, incarnation_of, split_sequence, HttpsRequest, HttpsResponse, HttpsTransport,
    ACK_HEADER, CONTENT_TYPE, EPOCH_HEADER, ETAG_HEADER, HOLD_HEADER, IF_MATCH_HEADER, MARK_HEADER,
};
use sce_rust_mesh::https::{JOINING_HEADER, JOINING_PROOF};
use sce_rust_mesh::link::LinkEvent;
use sce_rust_mesh::peers::{Machine, Members, OpenPeer, Peer};
use sce_rust_mesh::pool::{
    Fate, Pool, PoolConfig, SessionEndpoint, SessionFactory, SessionMachine,
};
use sce_rust_mesh::router::{PeerConfig, Responders};
use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_session_store::{MembershipStore, SqliteStore};
use sce_server_suite::clock::WallClock;
use sce_server_suite::ledger::{self, take_data, taken_by_key, Ledgers, HTTPS, TAKE, TAKEN};
use sce_server_suite::scratch::Scratch;
use sce_server_suite::wire;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

/// The token every instance proves its name with.
const TOKEN: &str = "suite-token";

/// The open machine whose instances post to the ledger.
const PHONE: &str = "phone";

/// A pool of the machine `F` builds with members, sending through the server's mailboxes.
type PoolOf<F> = Pool<F, MailboxTransport, WallClock>;

/// The pool of the ledger with members.
type OpenPool = PoolOf<Ledgers>;

/// The ledger bound to the instances of [`PHONE`], each a responder of its own request, held to
/// `members`.
fn open_ledger(members: Members) -> Machine {
    let base = ledger::machine(HTTPS, 600_000);
    let open: &'static [OpenPeer] = Box::leak(Box::new([OpenPeer {
        peer: Peer {
            name: PHONE,
            config: PeerConfig {
                responders: Responders::Instance,
                ..base.peers[0].config
            },
            // docs/adr/0113, D5: the instances send under `receipt: durable`, whose guarantee is
            // the completion records a repeat is answered from.
            link: ledger::durable_https(1_024, 600_000).link,
        },
        members,
    }]));
    Machine {
        peers: &[],
        open_peers: open,
        ..base
    }
}

/// The host the binding tells what an instance's exchange carried: it hands each envelope to the
/// pool with the edge's judgement and runs the session it names, and answers `200` only once each
/// is settled, `412` when a session found one fenced.
struct InstancesHost<F: SessionFactory<MailboxTransport, WallClock>> {
    pool: Arc<OnceLock<Arc<PoolOf<F>>>>,
    received: Arc<AtomicUsize>,
}

impl<F: SessionFactory<MailboxTransport, WallClock>> InstancesHost<F> {
    fn run(pool: &PoolOf<F>, keys: Vec<String>) {
        for key in keys {
            pool.run(&key);
        }
    }
}

impl<F: SessionFactory<MailboxTransport, WallClock> + 'static> ServerHost for InstancesHost<F> {
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
        let Some(pool) = self.pool.get() else {
            return Durability::NotDurable;
        };
        for event in events {
            match event {
                LinkEvent::Ready(peer) => Self::run(pool, pool.peer_ready(&peer)),
                LinkEvent::Lost(peer, _) => Self::run(pool, pool.peer_not_ready(&peer)),
                // Only instances post here, and each exchange of one is told with its judgement.
                _ => {}
            }
        }
        Durability::Now
    }

    fn deliver_member(
        &self,
        member: &str,
        events: Vec<LinkEvent>,
        judged: Judgement,
    ) -> Durability {
        let Some(pool) = self.pool.get() else {
            return Durability::NotDurable;
        };
        if judged.confirmed {
            Self::run(pool, pool.stand(member, judged.standing));
        }
        let mut receipts = Vec::new();
        for event in events {
            let keys = match event {
                LinkEvent::Ready(peer) => pool.peer_ready(&peer),
                LinkEvent::Lost(peer, _) => pool.peer_not_ready(&peer),
                // docs/adr/0116, D1 and D3: with the person the edge read for the incarnation.
                LinkEvent::Received(peer, bytes) => {
                    self.received.fetch_add(1, Ordering::SeqCst);
                    let (key, receipt) = pool.receive_member_for(
                        &peer,
                        judged.incarnation,
                        judged.standing,
                        judged.acting.clone(),
                        bytes,
                    );
                    receipts.push(receipt);
                    key.into_iter().collect()
                }
                _ => Vec::new(),
            };
            Self::run(pool, keys);
        }
        let mut durability = Durability::Now;
        for receipt in &receipts {
            match receipt.fate() {
                Fate::Settled => {}
                Fate::Fenced { current } => return Durability::Fenced { current },
                Fate::Pending | Fate::Lost => durability = Durability::NotDurable,
            }
        }
        durability
    }
}

/// A server on a loopback port with the ledger behind it, whose members are held to `members`.
struct Served {
    // Held so that the server runs for as long as the test does.
    _runtime: Runtime,
    address: SocketAddr,
    pool: Arc<OpenPool>,
    store: Arc<SqliteStore>,
    received: Arc<AtomicUsize>,
    port: HyperTransport,
    // Last, so that the store's directory is removed after everything that holds the store.
    _scratch: Scratch,
}

impl Served {
    fn start(name: &str, members: Members) -> Served {
        let scratch = Scratch::new(name);
        let store = Arc::new(SqliteStore::open(scratch.store()).expect("the reference store"));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("the suite's runtime");
        let machine = open_ledger(members);
        let pool: Arc<OnceLock<Arc<OpenPool>>> = Arc::new(OnceLock::new());
        let received = Arc::new(AtomicUsize::new(0));
        let host = InstancesHost {
            pool: Arc::clone(&pool),
            received: Arc::clone(&received),
        };
        let server = HttpsServer::new(
            ServerSettings {
                max_hold: Duration::from_millis(200),
                rates: sce_rust_mesh::peers::EdgeRates::NONE,
                ..ServerSettings::default()
            },
            Authenticator::Bearer(Arc::new(|_: &str, token: &str| token == TOKEN)),
            |name: &str| {
                sce_rust_mesh::peers::instance_of(name).is_some_and(|(machine, _)| machine == PHONE)
            },
            host,
        )
        .expect("the server's mailbox incarnation");
        server.keep_members(Arc::clone(&store) as Arc<dyn MembershipStore>, &machine);
        let served = Pool::with_members(
            machine,
            PoolConfig {
                owner: "server".to_string(),
                lease_ttl_ms: 600_000,
                renew_every_ms: 300_000,
            },
            Ledgers::default(),
            Arc::clone(&store) as Arc<dyn MembershipStore>,
            server.transport(),
            WallClock::default(),
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
        let served = Arc::new(served);
        assert!(
            pool.set(Arc::clone(&served)).is_ok(),
            "the pool is set once"
        );
        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        runtime.spawn(async move {
            let _ = server.serve(listener, None).await;
        });
        let port = HyperTransport::new(AdapterSettings {
            proxy: ProxyPolicy::Direct,
            ..AdapterSettings::default()
        })
        .expect("the reference adapter");
        Served {
            _runtime: runtime,
            address,
            pool: served,
            store,
            received,
            port,
            _scratch: scratch,
        }
    }

    /// An exchange of the instance `instance` carrying `body`, with the nonce `nonce`, under
    /// the incarnation `under` with the mark `mark` when it names one, and a join when not;
    /// acknowledging `ack` of its mailbox, numbered in `epoch`.
    fn exchange(
        &self,
        instance: &str,
        body: Vec<u8>,
        nonce: &str,
        under: Option<(u64, u64)>,
        ack: u64,
        epoch: Option<&str>,
    ) -> HttpsResponse {
        let mut headers = vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), ack.to_string()),
            (HOLD_HEADER.to_string(), "0".to_string()),
            ("Authorization".to_string(), format!("Bearer {TOKEN}")),
            (NONCE_HEADER.to_string(), nonce.to_string()),
        ];
        if let Some(epoch) = epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.to_string()));
        }
        if let Some((incarnation, mark)) = under {
            headers.push((IF_MATCH_HEADER.to_string(), entity_tag(incarnation)));
            headers.push((MARK_HEADER.to_string(), mark.to_string()));
        }
        self.port
            .exchange(HttpsRequest {
                url: format!("http://{}/sce-mesh/1/{PHONE}/{instance}", self.address),
                headers,
                body,
                timeout: Duration::from_secs(30),
            })
            .expect("a response")
    }

    /// The incarnation a join of `instance` with `nonce` is offered.
    fn join(&self, instance: &str, nonce: &str) -> u64 {
        let offered = self.exchange(instance, Vec::new(), nonce, None, 0, None);
        assert_eq!(offered.status, 200, "a join is answered");
        offered
            .header(ETAG_HEADER)
            .and_then(incarnation_of)
            .expect("a join is offered an incarnation")
    }

    /// The tags the store holds under each of `keys`.
    fn taken(&self, keys: &[String]) -> BTreeMap<String, Vec<String>> {
        taken_by_key(&*self.store, keys.iter().cloned()).expect("the store reads")
    }
}

/// The request `take` of the instance `instance`, numbered `number` in its incarnation, for
/// the session `key` with `tag`, under the ids `id` and `wire_id`.
fn take(instance: &str, number: u64, key: &str, tag: &str, id: u64) -> (Vec<u8>, [u8; 16]) {
    let mut envelope_id = [0x51u8; 16];
    envelope_id[8..].copy_from_slice(&id.to_be_bytes());
    let mut wire_id = [0x52u8; 16];
    wire_id[8..].copy_from_slice(&id.to_be_bytes());
    let data = take_data(key, tag, &[]);
    let bytes = Envelope {
        id: (&envelope_id).into(),
        source: Bounded::new(PHONE).expect("the machine's name fits an envelope"),
        instance: Some(Bounded::new(instance).expect("an instance id fits an envelope")),
        event_type: Bounded::new(TAKE).expect("an event name fits an envelope"),
        pattern: PatternKind::RpcRequest,
        datacontenttype: PayloadCodec::Json,
        data: Bounded::new(data.as_bytes()).expect("a request's data fits an envelope"),
        invoke_id: Some((&wire_id).into()),
        sequence_no: Some(number),
        ..Envelope::new()
    }
    .encode_to_vec();
    (bytes, wire_id)
}

/// The replies to `wire_id` a response carries.
fn replies_to(response: &HttpsResponse, wire_id: [u8; 16]) -> usize {
    split_sequence(&response.body)
        .into_iter()
        .filter_map(wire::read)
        .filter(|seen| seen.event == TAKEN && seen.wire_id == Some(wire_id) && seen.is_ok_reply())
        .count()
}

/// docs/adr/0113, D1 to D6: two hundred instances no deployment lists, spread over twenty
/// sessions, each join, confirm the incarnation they were offered with their first request, and
/// are answered; each request takes effect once. Red if the server binds only listed names, or
/// the pool refuses an instance's envelope.
#[test]
fn two_hundred_instances_never_listed_are_each_answered_once() {
    const INSTANCES: u64 = 200;
    let served = Served::start(
        "open-instances-two-hundred",
        Members {
            lease_ms: 600_000,
            provisional_ms: 600_000,
            max_instances: 256,
        },
    );
    let keys: Vec<String> = (0..20).map(|n| format!("k{n}")).collect();
    for n in 0..INSTANCES {
        let instance = format!("i{n}");
        let incarnation = served.join(&instance, &format!("join-{n}"));
        let key = &keys[(n % 20) as usize];
        let (bytes, wire_id) = take(&instance, 1, key, &format!("t{n}"), n);
        let answered = served.exchange(
            &instance,
            bytes,
            &format!("first-{n}"),
            Some((incarnation, 1)),
            0,
            None,
        );
        assert_eq!(answered.status, 200, "instance {instance}");
        assert_eq!(replies_to(&answered, wire_id), 1, "instance {instance}");
    }
    assert_eq!(served.received.load(Ordering::SeqCst), INSTANCES as usize);
    let taken = served.taken(&keys);
    let mut tags: Vec<String> = taken.values().flatten().cloned().collect();
    tags.sort();
    let mut expected: Vec<String> = (0..INSTANCES).map(|n| format!("t{n}")).collect();
    expected.sort();
    assert_eq!(tags, expected, "each request took effect once");
}

/// docs/adr/0113, D5, and docs/formal/OpenInstances.tla, `OpenInstances_without_records`: a
/// response lost on its way leaves the instance waiting, and the request asked again, the same
/// bytes under the same incarnation and mark, is answered from its completion record and not run
/// again, however many of its other requests came between. Red if a repeat runs again: the tag is
/// taken twice.
#[test]
fn a_request_whose_response_was_lost_is_answered_from_its_record() {
    let served = Served::start(
        "open-instances-lost-response",
        Members {
            lease_ms: 600_000,
            provisional_ms: 600_000,
            max_instances: 8,
        },
    );
    let incarnation = served.join("a1", "join");
    let (first, first_id) = take("a1", 1, "k", "lost", 1);
    // The response to this exchange is lost: the instance never reads it.
    let _lost = served.exchange("a1", first.clone(), "n1", Some((incarnation, 1)), 0, None);
    for number in 2..=20 {
        let (bytes, _) = take("a1", number, "k", &format!("between-{number}"), number);
        let answered = served.exchange(
            "a1",
            bytes,
            &format!("n{number}"),
            Some((incarnation, 1)),
            0,
            None,
        );
        assert_eq!(answered.status, 200);
    }
    let again = served.exchange("a1", first, "again", Some((incarnation, 1)), 0, None);
    assert_eq!(again.status, 200);
    assert!(
        replies_to(&again, first_id) >= 1,
        "answered from its record"
    );
    let taken = served.taken(&["k".to_string()]);
    assert_eq!(
        taken["k"].iter().filter(|tag| *tag == "lost").count(),
        1,
        "the request took effect once"
    );
}

/// docs/adr/0113, D4, and docs/formal/OpenInstances.tla, `OpenInstances_without_fence`: an
/// instance whose lease ran out is no member; its next exchange under the incarnation it held is
/// fenced `412` and takes nothing in, and a join offers it a new incarnation under which it is
/// answered again. Red if an exchange of an ended incarnation is taken in: its tag is kept.
#[test]
fn an_instance_whose_lease_ran_out_is_fenced_and_joins_again() {
    let served = Served::start(
        "open-instances-expired-lease",
        Members {
            lease_ms: 1_000,
            provisional_ms: 1_000,
            max_instances: 8,
        },
    );
    let first = served.join("a1", "join-1");
    let (bytes, wire_id) = take("a1", 1, "k", "kept", 1);
    let answered = served.exchange("a1", bytes, "n1", Some((first, 1)), 0, None);
    assert_eq!(answered.status, 200);
    assert_eq!(replies_to(&answered, wire_id), 1);

    std::thread::sleep(Duration::from_millis(1_500));
    for key in served.pool.end_members() {
        served.pool.run(&key);
    }
    let (late, _) = take("a1", 2, "k", "fenced", 2);
    let fenced = served.exchange("a1", late, "n2", Some((first, 2)), 0, None);
    assert_eq!(fenced.status, 412, "an incarnation that ran out is fenced");

    let second = served.join("a1", "join-2");
    assert_ne!(second, first, "a join after the end is offered anew");
    let (again, again_id) = take("a1", 1, "k", "after", 3);
    let answered = served.exchange("a1", again, "n3", Some((second, 1)), 0, None);
    assert_eq!(answered.status, 200);
    assert_eq!(replies_to(&answered, again_id), 1);
    let taken = served.taken(&["k".to_string()]);
    assert_eq!(taken["k"], vec!["kept".to_string(), "after".to_string()]);
}

/// The key that signs the proofs of the people behind instances, and the issuer they name.
const PROOF_KEY: [u8; 32] = [7; 32];
const ISSUER: &str = "https://id.example";

/// What the sessions of [`Witnesses`] ran: each event, with the subject of the person it was
/// said for.
type Seen = Arc<std::sync::Mutex<Vec<(String, Option<String>)>>>;

/// A session that records the person each event it runs was said for, which is what a document
/// that declares `sce:principal` reads as `_x.principal` (docs/adr/0116, D3), and answers
/// nothing. Written by hand in the shape a generated machine has to the pool, as the ledger is.
struct Witness {
    seen: Seen,
}

impl SessionMachine for Witness {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            if let EngineCall::Raise(event) = call {
                stepped = true;
                let person = event
                    .metadata
                    .principal
                    .as_ref()
                    .map(|person| person.subject.clone());
                self.seen
                    .lock()
                    .expect("what was seen")
                    .push((event.name.clone(), person));
            }
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "witness".to_string(),
            configuration: vec!["witnessing".to_string()],
            current: "witnessing".to_string(),
            variables: Vec::new(),
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "witness".to_string(),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_starts: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
            features: None,
        })
    }

    fn finished(&self) -> bool {
        false
    }
}

/// The factory of witnesses, each recording into one list.
struct Witnesses {
    seen: Seen,
}

impl SessionFactory<MailboxTransport, WallClock> for Witnesses {
    type Machine = Witness;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<MailboxTransport, WallClock>,
        _wall_now_ms: u64,
    ) -> Result<Witness, String> {
        Ok(Witness {
            seen: Arc::clone(&self.seen),
        })
    }

    fn restore(
        &self,
        _key: &str,
        _state: &SavedState,
        _endpoint: &SessionEndpoint<MailboxTransport, WallClock>,
        _wall_now_ms: u64,
    ) -> Result<Witness, String> {
        Ok(Witness {
            seen: Arc::clone(&self.seen),
        })
    }
}

/// Milliseconds since the Unix epoch.
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// docs/adr/0116, D1 and D3, end to end over the binding: an instance whose join says a proof
/// follows is confirmed by a proof of `alice`, and the event its next request raises in the
/// pool's session is said for her, as the server's edge read the delegation its store holds for
/// the incarnation, whatever the request carries. Red if the edge hands its host no person
/// (`Judgement::acting`), or the pool takes the envelope in without it
/// (`Pool::receive_member_for`, `Endpoint::receive_member_for`).
#[test]
fn an_instance_acts_for_the_person_its_proof_bound_in_the_pools_session() {
    let scratch = Scratch::new("open-instances-person");
    let store = Arc::new(SqliteStore::open(scratch.store()).expect("the reference store"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("the suite's runtime");
    let machine = open_ledger(Members {
        lease_ms: 600_000,
        provisional_ms: 600_000,
        max_instances: 8,
    });
    let server_name = machine.name;
    let pool: Arc<OnceLock<Arc<PoolOf<Witnesses>>>> = Arc::new(OnceLock::new());
    let seen: Seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let server = HttpsServer::new(
        ServerSettings {
            max_hold: Duration::from_millis(200),
            rates: sce_rust_mesh::peers::EdgeRates::NONE,
            ..ServerSettings::default()
        },
        Authenticator::Key(KeyPins::first_use(Arc::new(MemoryCredentials::new()))),
        |name: &str| {
            sce_rust_mesh::peers::instance_of(name).is_some_and(|(machine, _)| machine == PHONE)
        },
        InstancesHost {
            pool: Arc::clone(&pool),
            received: Arc::new(AtomicUsize::new(0)),
        },
    )
    .expect("the server's mailbox incarnation");
    server.keep_members(Arc::clone(&store) as Arc<dyn MembershipStore>, &machine);
    server.keep_principals(
        Arc::new(TestProof::new(PROOF_KEY, ISSUER, server_name, 60_000)),
        Login {
            login_ms: 30_000,
            person_ttl_ms: 600_000,
        },
    );
    let witnessed = Pool::with_members(
        machine,
        PoolConfig {
            owner: "server".to_string(),
            lease_ttl_ms: 600_000,
            renew_every_ms: 300_000,
        },
        Witnesses {
            seen: Arc::clone(&seen),
        },
        Arc::clone(&store) as Arc<dyn MembershipStore>,
        server.transport(),
        WallClock::default(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    assert!(
        pool.set(Arc::new(witnessed)).is_ok(),
        "the pool is set once"
    );
    let listener = runtime
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("a loopback port");
    let address = listener.local_addr().expect("its address");
    runtime.spawn(async move {
        let _ = server.serve(listener, None).await;
    });
    let port = HyperTransport::new(AdapterSettings {
        proxy: ProxyPolicy::Direct,
        ..AdapterSettings::default()
    })
    .expect("the reference adapter");

    let signer = RequestSigner::new(sce_rust_crypto::ed25519::KeyPair::from_seed(&[1u8; 32]));
    let key_id = signer.key_id().to_string();
    let url = format!("http://{address}/sce-mesh/1/{PHONE}/a1");
    let target = normalize_target_uri(&url).expect("a URL");
    let send = |nonce: &str, body: Vec<u8>, extra: &[(&str, String)]| {
        let mut headers = vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), "0".to_string()),
            (HOLD_HEADER.to_string(), "0".to_string()),
            (NONCE_HEADER.to_string(), nonce.to_string()),
            (CONTENT_DIGEST_HEADER.to_string(), content_digest(&body)),
        ];
        headers.extend(
            extra
                .iter()
                .map(|(name, value)| (name.to_string(), value.clone())),
        );
        let seconds = i64::try_from(unix_ms() / 1_000).unwrap_or(i64::MAX);
        let signed = signer
            .sign(&target, &headers, seconds)
            .expect("every covered field");
        headers.extend(signed);
        port.exchange(HttpsRequest {
            url: url.clone(),
            headers,
            body,
            timeout: Duration::from_secs(30),
        })
        .expect("a response")
    };

    let offered = send(
        "join",
        Vec::new(),
        &[(JOINING_HEADER, JOINING_PROOF.to_string())],
    );
    assert_eq!(offered.status, 200, "a join that says a proof follows");
    let offer = offered
        .header(ETAG_HEADER)
        .and_then(incarnation_of)
        .expect("an offer");
    let under = [
        (IF_MATCH_HEADER, entity_tag(offer)),
        (MARK_HEADER, "1".to_string()),
    ];
    let proof = TestProof::make(
        &PROOF_KEY,
        ISSUER,
        server_name,
        "alice",
        &Challenge::of(&Offering {
            server: server_name,
            machine: PHONE,
            instance: "a1",
            key: &key_id,
            incarnation: offer,
        }),
        unix_ms(),
    );
    assert_eq!(
        send("proof", proof, &under).status,
        200,
        "the proof confirms"
    );
    let (bytes, _) = take("a1", 1, "k", "said", 1);
    assert_eq!(send("take", bytes, &under).status, 200);
    let seen = seen.lock().expect("what was seen");
    let taken: Vec<&Option<String>> = seen
        .iter()
        .filter(|(event, _)| event == TAKE)
        .map(|(_, person)| person)
        .collect();
    assert_eq!(taken, [&Some("alice".to_string())], "{seen:?}");
}
