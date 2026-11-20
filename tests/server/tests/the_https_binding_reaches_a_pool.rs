// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A keyed pool reached over the HTTPS binding (docs/adr/0107, decision D10, its fifth bullet;
//! D2 with D3, D4 and D5): through an explicit proxy, through a proxy that inspects TLS under an
//! authority the client's platform trusts, with a response lost on purpose, and with the store
//! refusing the checkpoint that would hold a request.
//!
//! The server is the binding's server (`HttpsServer`) in front of the ledger served as a keyed
//! pool on the reference store, its host handing the pool each envelope an exchange carried and
//! running the session it names before the exchange is answered, and answering `200` only once a
//! durable checkpoint holds every envelope the exchange carried (the envelopes' receipts,
//! SCE_MESH.md section 10.11); the client is a machine's endpoint on the binding's client and its
//! reference adapter (`HyperTransport`). The proxies and the test authorities are the conformance
//! kit's (SCE_MESH.md section 22.7), so the pool is reached through the very intermediaries every
//! adapter is held to. An authority the client is told to believe beside the operating system's
//! stands for one the platform trusts: the adapter believes both alike.

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{Endpoint, EngineCall, MESH_RPC_INVOKE_TYPE};
use sce_rust_mesh::https::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};
use sce_rust_mesh::https::client::{ClientSettings, HttpsClient, HttpsClientTransport};
use sce_rust_mesh::https::conformance::{
    inspecting_proxy, tunnelling_proxy, ProxyLog, TestAuthority,
};
use sce_rust_mesh::https::server::{
    server_tls, Authenticator, Durability, HttpsServer, MailboxTransport, ServerHost,
    ServerSettings,
};
use sce_rust_mesh::https::{Credential, HttpsFailure, HttpsRequest, HttpsResponse, HttpsTransport};
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::link::{deliver, LinkEvent};
use sce_rust_mesh::outbound::RetryPolicy;
use sce_rust_mesh::peers::EdgeRates;
use sce_rust_mesh::pool::{Fate, Pool, PoolConfig};
use sce_rust_mesh::reconnect::Backoff;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::rpc::MESH_EVENT_PARAM;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::HostInvokeRequest;
use sce_rust_session_store::{
    AcquireError, Change, Due, Failure, Lease, LeaseError, SessionStore, SqliteStore, Stored,
    Write, WriteError, Written,
};
use sce_server_suite::clock::WallClock;
use sce_server_suite::ledger::{self, take_data, taken_by_key, Ledgers, CLIENT, HTTPS, TAKE};
use sce_server_suite::scratch::Scratch;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;

/// The token the client proves its name with.
const TOKEN: &str = "suite-token";

/// How long any one expectation is waited for.
const PATIENCE: Duration = Duration::from_secs(30);

/// The pool behind the binding.
type HttpsPool = Pool<Ledgers, MailboxTransport, WallClock>;

/// The reference store, made to refuse its writes on demand as a medium that is down refuses
/// them: while `refusing` is set, every checkpoint the pool writes fails and nothing is written.
struct Refusing {
    inner: SqliteStore,
    refusing: AtomicBool,
    /// How many writes it refused.
    refused: AtomicUsize,
}

impl SessionStore for Refusing {
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        self.inner.get(key)
    }

    /// A checkpoint (a write that keeps the session) is refused while the medium is down; a
    /// removal is passed on as it is.
    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        let kept = matches!(write.change, Change::Keep { .. });
        if kept && self.refusing.load(Ordering::SeqCst) {
            self.refused.fetch_add(1, Ordering::SeqCst);
            return Err(WriteError::Failed(Failure::new("the medium is down")));
        }
        self.inner.write(key, write)
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        self.inner.due_before(ms, limit)
    }

    fn indexed(&self, key: &str) -> String {
        self.inner.indexed(key)
    }

    fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
        self.inner.key_of(indexed)
    }

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        self.inner.indexed_after(after, limit)
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        self.inner.acquire(key, owner, ttl_ms, now_ms)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        self.inner.renew(key, fence, ttl_ms, now_ms)
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        self.inner.release(key, fence)
    }
}

/// The host the binding tells what an exchange carried: it hands each envelope to the pool and
/// runs the session it names, which saves it, before the exchange is answered, and lets a `200`
/// acknowledge the envelopes only once each is settled, a durable checkpoint holding what its
/// session made of it (SCE_MESH.md section 10.11).
struct PoolHost {
    pool: Arc<OnceLock<HttpsPool>>,
    /// How many envelopes the binding handed the pool.
    received: Arc<AtomicUsize>,
    /// What the pool told its host it could not act on, a session's endpoint's errors among it
    /// (a source its link does not name, SCE_MESH.md section 10.9, invariant 10), as words.
    errors: Arc<Mutex<Vec<String>>>,
}

impl ServerHost for PoolHost {
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
        let Some(pool) = self.pool.get() else {
            // No pool took anything in.
            return Durability::NotDurable;
        };
        let mut receipts = Vec::new();
        for event in events {
            let run = match event {
                LinkEvent::Ready(peer) => pool.peer_ready(&peer),
                LinkEvent::Lost(peer, _) => pool.peer_not_ready(&peer),
                LinkEvent::Received(peer, bytes) => {
                    self.received.fetch_add(1, Ordering::SeqCst);
                    let (key, receipt) = pool.receive_with_receipt(&peer, bytes);
                    receipts.push(receipt);
                    key.into_iter().collect()
                }
                // The accepting side hands back nothing it could not write: what the pool sends
                // waits in the client's mailbox. Nor does it tell an interruption, a restart or an
                // answer that did not verify, which are what a client observes of its server.
                LinkEvent::Unsent(..)
                | LinkEvent::Interrupted(..)
                | LinkEvent::Restarted(_)
                | LinkEvent::Unauthenticated(..) => Vec::new(),
            };
            for key in run {
                pool.run(&key);
            }
        }
        self.errors
            .lock()
            .expect("the server's errors")
            .extend(pool.take_errors().iter().map(|error| format!("{error:?}")));
        // Each session the envelopes named has run, and its run settled what it took in: held by
        // a durable checkpoint, or lost or still unwritten, which no `200` may acknowledge.
        if receipts
            .iter()
            .all(|receipt| receipt.fate() == Fate::Settled)
        {
            Durability::Now
        } else {
            Durability::NotDurable
        }
    }
}

/// A server on a loopback port, under a certificate a test authority issued, with the ledger
/// behind it as a keyed pool.
struct Served {
    runtime: Runtime,
    authority: TestAuthority,
    address: SocketAddr,
    /// How many envelopes the binding handed the pool.
    received: Arc<AtomicUsize>,
    /// What the pool told its host it could not act on, as words.
    errors: Arc<Mutex<Vec<String>>>,
    store: Arc<Refusing>,
    // Last, so that the store's directory is removed after everything that holds the store.
    _scratch: Scratch,
}

impl Served {
    fn start(name: &str) -> Served {
        let scratch = Scratch::new(name);
        let store = Arc::new(Refusing {
            inner: SqliteStore::open(scratch.store()).expect("the reference store"),
            refusing: AtomicBool::new(false),
            refused: AtomicUsize::new(0),
        });
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("the suite's runtime");
        let authority = TestAuthority::new("SCE server suite authority");
        let (chain, key) = authority.issue("127.0.0.1");
        let tls = server_tls(chain, key, None).expect("the server's TLS");

        let pool: Arc<OnceLock<HttpsPool>> = Arc::new(OnceLock::new());
        let received = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let host = PoolHost {
            pool: Arc::clone(&pool),
            received: Arc::clone(&received),
            errors: Arc::clone(&errors),
        };
        let server = HttpsServer::new(
            ServerSettings {
                max_hold: Duration::from_secs(2),
                // As the binding it serves declares (ledger::HTTPS): this scenario judges what the
                // server keeps, and tests/the_https_edge_limits_rate.rs judges the edge's rates.
                rates: EdgeRates::NONE,
                ..ServerSettings::default()
            },
            Authenticator::Bearer(Arc::new(|name: &str, token: &str| {
                name == CLIENT && token == TOKEN
            })),
            |name: &str| name == CLIENT,
            host,
        )
        .expect("the server's mailbox incarnation");
        let served = Pool::new(
            ledger::machine(HTTPS, 600_000),
            PoolConfig {
                owner: "server".to_string(),
                lease_ttl_ms: 600_000,
                renew_every_ms: 300_000,
            },
            Ledgers::default(),
            Arc::clone(&store) as Arc<dyn SessionStore>,
            server.transport(),
            WallClock::default(),
        )
        .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
        assert!(pool.set(served).is_ok(), "the pool is set once");

        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        runtime.spawn(async move {
            let _ = server.serve(listener, Some(tls)).await;
        });
        Served {
            runtime,
            authority,
            address,
            received,
            errors,
            store,
            _scratch: scratch,
        }
    }

    fn base(&self) -> String {
        format!("https://{}", self.address)
    }

    fn received(&self) -> usize {
        self.received.load(Ordering::SeqCst)
    }

    /// Make the store refuse every write from now on, or write again.
    fn refuse_writes(&self, refusing: bool) {
        self.store.refusing.store(refusing, Ordering::SeqCst);
    }

    /// How many writes the store refused.
    fn refused(&self) -> usize {
        self.store.refused.load(Ordering::SeqCst)
    }

    /// The tags each of `keys` holds, read from the store.
    fn taken(&self, keys: &[&str]) -> BTreeMap<String, Vec<String>> {
        taken_by_key(&*self.store, keys.iter().map(|key| key.to_string())).expect("the store reads")
    }
}

/// The reference adapter, believing `authorities` beside the operating system's, reaching every
/// server through the proxy at `proxy`.
fn adapter(authorities: &[Vec<u8>], proxy: SocketAddr) -> HyperTransport {
    HyperTransport::new(AdapterSettings {
        extra_authorities: authorities.to_vec(),
        proxy: ProxyPolicy::Through(format!("http://{proxy}")),
        client_identity: None,
    })
    .expect("the reference adapter")
}

/// What the server answered one exchange that carried envelopes: its status, and whether the
/// store held the watched tag when the answer arrived.
type Answer = (u16, bool);

/// A port that carries each exchange on the reference adapter, and notes for each that carried
/// envelopes what the server answered and whether the store held `tag` under `key` by then.
struct Watched {
    inner: HyperTransport,
    store: Arc<Refusing>,
    key: &'static str,
    tag: &'static str,
    answers: Arc<Mutex<Vec<Answer>>>,
}

impl HttpsTransport for Watched {
    fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
        let carried = !request.body.is_empty();
        let outcome = self.inner.exchange(request);
        if let (true, Ok(response)) = (carried, &outcome) {
            let taken =
                taken_by_key(&*self.store, [self.key.to_string()]).expect("the store reads");
            let held = taken[self.key].iter().any(|tag| tag == self.tag);
            self.answers
                .lock()
                .expect("the answers")
                .push((response.status, held));
        }
        outcome
    }
}

/// How a request the client asked ended.
#[derive(Debug, Clone)]
struct Ended {
    invoke_id: String,
    failed: bool,
    data: String,
}

/// The client machine: an endpoint on the binding's client, asking the ledger with
/// `<invoke type="sce:mesh-rpc">`.
struct Client {
    endpoint: Endpoint<HttpsClientTransport, WallClock>,
    client: HttpsClient,
    events: mpsc::Receiver<LinkEvent>,
    ready: bool,
    ended: Vec<Ended>,
    token: u64,
    /// What the client's endpoint told its host it could not act on, and the rows it raised for
    /// its machine, as words.
    host_errors: Vec<String>,
    /// What the server's pool told its host, as words ([`Served`]).
    server_errors: Arc<Mutex<Vec<String>>>,
    /// While `Some`, what arrives from the server is held back here, in order, until a failed
    /// exchange has been applied whole: the envelope it hands back, and the end of readiness it
    /// brings when the link was ready. A scenario that loses a response sets it, so the client
    /// machine is told of the failure before any reply, whichever the network delivered first.
    held: Option<Vec<LinkEvent>>,
    /// Whether an exchange handed an envelope back while arrivals were held.
    failed: bool,
}

impl Client {
    /// A client of `served`'s server, through `port`.
    fn start(
        served: &Served,
        port: impl HttpsTransport + 'static,
        retry: Option<RetryPolicy>,
    ) -> Client {
        let base = served.base();
        let (tx, events) = mpsc::channel();
        let settings = ClientSettings {
            hold: Duration::from_secs(1),
            exchange_timeout: Duration::from_secs(5),
            backoff: Backoff {
                initial: Duration::from_millis(50),
                max: Duration::from_millis(500),
                factor: 2,
                jitter: false,
            },
            ..ClientSettings::new(
                base,
                CLIENT,
                ledger::MACHINE_NAME,
                Credential::Bearer(TOKEN.to_string()),
            )
        };
        let client = HttpsClient::start(settings, Arc::new(port), move |event| {
            let _ = tx.send(event);
        })
        .expect("a client the binding can post under");
        let mut router = Router::new(CLIENT, 64, 50).expect("a router");
        router.add_peer(
            ledger::MACHINE_NAME,
            PeerConfig {
                transport: "https",
                buffer: None,
                retry,
                stamp_sequence: false,
                delivery: Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &[ledger::MACHINE_NAME],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
        );
        let endpoint = Endpoint::new(router, client.transport(), WallClock::default());
        Client {
            endpoint,
            client,
            events,
            ready: false,
            ended: Vec::new(),
            token: 0,
            host_errors: Vec::new(),
            server_errors: Arc::clone(&served.errors),
            held: None,
            failed: false,
        }
    }

    /// Hold back what arrives from the server until an exchange has failed and its failure has
    /// been applied (see [`Client::held`]).
    fn hold_arrivals_until_a_failure(&mut self) {
        self.held = Some(Vec::new());
        self.failed = false;
    }

    /// Hand `event` to the client machine, or hold it back while [`Client::held`] says so.
    fn hand_on(&mut self, event: LinkEvent) {
        let Some(held) = self.held.as_mut() else {
            deliver(&mut self.endpoint, event);
            return;
        };
        if matches!(event, LinkEvent::Received(..)) {
            held.push(event);
            return;
        }
        let failure = matches!(event, LinkEvent::Unsent(..));
        let edge = matches!(event, LinkEvent::Interrupted(..) | LinkEvent::Lost(..));
        deliver(&mut self.endpoint, event);
        self.failed |= failure;
        // Applied whole: the envelope handed back, then the end of readiness, which follows it
        // at once when the link was ready and does not come when it was not.
        if self.failed && (edge || (failure && !self.ready)) {
            for arrived in self.held.take().unwrap_or_default() {
                deliver(&mut self.endpoint, arrived);
            }
        }
    }

    /// Every host error either side was told, so that a scenario that stalls names its cause:
    /// the client's endpoint's, then the server's pool's.
    fn errors(&self) -> Vec<String> {
        let mut errors = self.host_errors.clone();
        errors.extend(
            self.server_errors
                .lock()
                .expect("the server's errors")
                .iter()
                .cloned(),
        );
        errors
    }

    /// Ask the session `key` to take `tag`, under the invoke id `tag`.
    fn ask(&mut self, key: &str, tag: &str) {
        self.token += 1;
        let mut params = HashMap::new();
        params.insert(MESH_EVENT_PARAM.to_string(), vec![TAKE.to_string()]);
        let started = self.endpoint.invoke(&HostInvokeRequest {
            processor_type: MESH_RPC_INVOKE_TYPE.to_string(),
            invoke_id: tag.to_string(),
            src: format!("#{}", ledger::MACHINE_NAME),
            params,
            event_data: take_data(key, tag, &[]),
            token: self.token,
            ..HostInvokeRequest::default()
        });
        assert!(started.refusal.is_none(), "{started:?}");
    }

    /// Drive the client machine (its link's events, its retries) until `done` holds of it, or
    /// panic with `what` after the suite's patience.
    fn until(&mut self, what: &str, done: impl Fn(&Client) -> bool) {
        let deadline = Instant::now() + PATIENCE;
        while !done(self) {
            assert!(
                Instant::now() < deadline,
                "waited too long for {what}; host errors: {:?}",
                self.errors()
            );
            if let Ok(event) = self.events.recv_timeout(Duration::from_millis(20)) {
                match &event {
                    LinkEvent::Ready(_) => self.ready = true,
                    LinkEvent::Lost(..) | LinkEvent::Interrupted(..) => self.ready = false,
                    LinkEvent::Received(..)
                    | LinkEvent::Unsent(..)
                    | LinkEvent::Restarted(_)
                    | LinkEvent::Unauthenticated(..) => {}
                }
                self.hand_on(event);
            }
            self.endpoint.tick();
            self.host_errors.extend(
                self.endpoint
                    .take_host_errors()
                    .iter()
                    .map(|error| format!("{error:?}")),
            );
            for call in self.endpoint.take_calls() {
                match call {
                    EngineCall::EndInvoke {
                        invoke_id,
                        failed,
                        data,
                        ..
                    } => self.ended.push(Ended {
                        invoke_id,
                        failed,
                        data,
                    }),
                    // A row the client's machine is told (a lost link, a request it lost with
                    // it), kept beside the host errors so that a stall names its cause.
                    EngineCall::Raise(event) => self
                        .host_errors
                        .push(format!("raised {}: {}", event.name, event.metadata.data)),
                }
            }
        }
    }

    /// Drive the client machine for `period`: long enough for a second copy of what arrived to
    /// arrive too, were one coming.
    fn settle(&mut self, period: Duration) {
        let until = Instant::now() + period;
        self.until("the period to pass", |_| Instant::now() >= until);
    }

    /// Stop the client; how its requests ended.
    fn stop(self) -> Vec<Ended> {
        self.client.stop();
        self.ended
    }
}

/// The position a `take` reply says its request stands at.
fn position(data: &str) -> Option<usize> {
    match json::parse(data).ok()?.member("position") {
        Some(Value::Number(digits)) => digits.parse().ok(),
        _ => None,
    }
}

/// Each of `asked` (a key and a tag) ended once, well, and its reply names where the store holds
/// its tag, which the store holds once.
fn each_answered_once_where_the_store_holds_it(
    served: &Served,
    asked: &[(&str, &str)],
    ended: &[Ended],
) {
    let keys: Vec<&str> = asked.iter().map(|(key, _)| *key).collect();
    let taken = served.taken(&keys);
    for (key, tag) in asked {
        let ends: Vec<&Ended> = ended.iter().filter(|end| end.invoke_id == *tag).collect();
        let [end] = ends.as_slice() else {
            panic!("{tag} ended {} times: {ended:?}", ends.len());
        };
        assert!(!end.failed, "{tag} failed: {end:?}");
        let held: Vec<usize> = taken[*key]
            .iter()
            .enumerate()
            .filter(|(_, held)| held == tag)
            .map(|(at, _)| at + 1)
            .collect();
        assert_eq!(
            held.len(),
            1,
            "{tag} is held {} times: {taken:?}",
            held.len()
        );
        assert_eq!(position(&end.data), Some(held[0]), "{end:?} {taken:?}");
    }
    for key in keys {
        let asked_of = asked.iter().filter(|(of, _)| *of == key).count();
        assert_eq!(taken[key].len(), asked_of, "{key} holds {:?}", taken[key]);
    }
}

/// What every scenario asks: two sessions, one of them twice.
const ASKED: [(&str, &str); 3] = [("k1", "k1.a"), ("k2", "k2.a"), ("k1", "k1.b")];

/// The client tunnels to the server through an explicit proxy with `CONNECT`, and the pool behind
/// the server answers each request once, from the state it saved.
#[test]
fn a_pool_answers_through_an_explicit_proxy() {
    let served = Served::start("explicit-proxy");
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = tunnelling_proxy(&served.runtime, Arc::clone(&log));
    let mut client = Client::start(&served, adapter(&[served.authority.der()], proxy), None);
    client.until("the server to be ready", |client| client.ready);
    for (key, tag) in ASKED {
        client.ask(key, tag);
    }
    client.until("every answer", |client| client.ended.len() >= ASKED.len());
    client.settle(Duration::from_millis(500));
    let ended = client.stop();

    each_answered_once_where_the_store_holds_it(&served, &ASKED, &ended);
    let log = log.lock().expect("the proxy's log");
    assert!(!log.tunnels.is_empty(), "nothing went through the proxy");
    let server = served.address.to_string();
    assert!(
        log.tunnels.iter().all(|target| *target == server),
        "{:?}",
        log.tunnels
    );
}

/// A proxy that inspects TLS answers the client's TLS under an authority of its own, which the
/// client's platform trusts, and reads every exchange in the clear on its way to the server; the
/// pool behind the server answers each request once, as it does with no one between.
#[test]
fn a_pool_answers_through_a_proxy_that_inspects_tls() {
    let served = Served::start("inspecting-proxy");
    let inspector = Arc::new(TestAuthority::new("SCE server suite inspecting authority"));
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = inspecting_proxy(
        &served.runtime,
        Arc::clone(&inspector),
        served.authority.der(),
        false,
        Arc::clone(&log),
    );
    let mut client = Client::start(&served, adapter(&[inspector.der()], proxy), None);
    client.until("the server to be ready", |client| client.ready);
    for (key, tag) in ASKED {
        client.ask(key, tag);
    }
    client.until("every answer", |client| client.ended.len() >= ASKED.len());
    client.settle(Duration::from_millis(500));
    let ended = client.stop();

    each_answered_once_where_the_store_holds_it(&served, &ASKED, &ended);
    let plaintext = String::from_utf8_lossy(&log.lock().expect("the proxy's log").plaintext)
        .to_ascii_lowercase();
    assert!(
        plaintext.contains(&format!("post /sce-mesh/1/{CLIENT} http/1.1")),
        "the proxy read no exchange in the clear"
    );
}

/// The proxy drops the response to the exchange that carried the request, after the pool took it
/// in and saved it. The client sends the request again under its retry policy; the session's
/// duplicate window knows it and answers it with the reply its first copy made (docs/adr/0107,
/// D4), so the session takes it once, and the client's machine sees one answer.
///
/// The reply reaches the client by the held poll, from the mailbox, or answering the copy, and
/// the network decides which comes before the failure of the exchange whose response was
/// dropped. The client machine is always told of the failure first (the request handed back,
/// the link interrupted) and of the reply after it, the order in which a link loss once let the
/// request go: the failed exchange interrupts the link and loses nothing, since the server keeps
/// the reply (SCE_MESH.md section 22.4), so the reply still ends the request.
#[test]
fn a_response_lost_after_the_pool_took_its_request_is_answered_once() {
    let served = Served::start("lost-response");
    let inspector = Arc::new(TestAuthority::new("SCE server suite dropping authority"));
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = inspecting_proxy(
        &served.runtime,
        Arc::clone(&inspector),
        served.authority.der(),
        true,
        Arc::clone(&log),
    );
    let retry = RetryPolicy {
        max_retries: 5,
        initial_backoff_ms: 50,
        backoff_multiplier: 2.0,
        max_backoff_ms: 1_000,
        jitter_pct: 0,
    };
    let mut client = Client::start(&served, adapter(&[inspector.der()], proxy), Some(retry));
    client.until("the server to be ready", |client| client.ready);
    client.hold_arrivals_until_a_failure();
    let asked = [("k1", "k1.lost")];
    client.ask(asked[0].0, asked[0].1);
    client.until("the answer", |client| !client.ended.is_empty());
    client.until("the copy sent again to reach the pool", |_| {
        served.received() >= 2
    });
    client.settle(Duration::from_millis(500));
    // Both sides name the machine at the other end of the link as the source of what they send
    // (SCE_MESH.md section 22.1): a reply carried again, from the mailbox or the reply cache, is
    // still the server machine's, and a request carried again is still the client's.
    let errors = client.errors();
    assert!(
        !errors.iter().any(|error| error.contains("SourceNotLink")),
        "{errors:?}"
    );
    let ended = client.stop();

    assert!(
        log.lock().expect("the proxy's log").dropped,
        "no response was dropped"
    );
    each_answered_once_where_the_store_holds_it(&served, &asked, &ended);
}

/// The store refuses the checkpoints of the session that takes a request in. The server answers
/// no exchange that carried the request `200` while no durable checkpoint holds it (SCE_MESH.md
/// section 10.11, docs/adr/0107 D5): it sheds each as `503`, and the client carries the request
/// again after the wait the server asked for. Once the store writes again, the next copy, which
/// the session's duplicate window drops, drives the checkpoint that holds the first, and that
/// exchange is answered `200`; the request is answered once, and taken in once.
#[test]
fn a_request_whose_checkpoint_the_store_refused_is_answered_once_it_is_durable() {
    let served = Served::start("refused-checkpoint");
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = tunnelling_proxy(&served.runtime, Arc::clone(&log));
    let asked = [("k1", "k1.refused")];
    let answers: Arc<Mutex<Vec<Answer>>> = Arc::new(Mutex::new(Vec::new()));
    let port = Watched {
        inner: adapter(&[served.authority.der()], proxy),
        store: Arc::clone(&served.store),
        key: asked[0].0,
        tag: asked[0].1,
        answers: Arc::clone(&answers),
    };
    let mut client = Client::start(&served, port, None);
    client.until("the server to be ready", |client| client.ready);

    served.refuse_writes(true);
    client.ask(asked[0].0, asked[0].1);
    client.until("two exchanges carrying the request to be shed", |_| {
        let answers = answers.lock().expect("the answers");
        answers.iter().filter(|(status, _)| *status == 503).count() >= 2
    });
    assert!(
        served.refused() >= 2,
        "the store refused {} writes",
        served.refused()
    );
    assert!(
        served.taken(&["k1"])["k1"].is_empty(),
        "the store holds nothing while it refuses"
    );
    served.refuse_writes(false);
    client.until("the answer", |client| !client.ended.is_empty());
    client.settle(Duration::from_millis(500));
    let ended = client.stop();

    let answers = answers.lock().expect("the answers").clone();
    let Some((last, shed)) = answers.split_last() else {
        panic!("no exchange carried the request");
    };
    assert_eq!(
        *last,
        (200, true),
        "the request is acknowledged once, after a durable checkpoint held it: {answers:?}"
    );
    assert!(
        shed.len() >= 2 && shed.iter().all(|(status, _)| *status == 503),
        "every exchange before it was shed: {answers:?}"
    );
    assert!(
        served.received() >= answers.len(),
        "each copy reached the pool: {} of {}",
        served.received(),
        answers.len()
    );
    each_answered_once_where_the_store_holds_it(&served, &asked, &ended);
}
