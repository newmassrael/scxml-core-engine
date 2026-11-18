// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A keyed pool reached over the HTTPS binding (docs/adr/0107, decision D10, its fifth bullet;
//! D2 with D3, D4 and D5): through an explicit proxy, through a proxy that inspects TLS under an
//! authority the client's platform trusts, and with a response lost on purpose.
//!
//! The server is the binding's server (`HttpsServer`) in front of the ledger served as a keyed
//! pool on the reference store, its host handing the pool each envelope an exchange carried and
//! running the session it names before the exchange is answered; the client is a machine's
//! endpoint on the binding's client and its reference adapter (`HyperTransport`). The proxies and
//! the test authorities are the conformance kit's (SCE_MESH.md section 22.7), so the pool is
//! reached through the very intermediaries every adapter is held to. An authority the client is
//! told to believe beside the operating system's stands for one the platform trusts: the adapter
//! believes both alike.

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sce_rust_mesh::endpoint::{Endpoint, EngineCall, MESH_RPC_INVOKE_TYPE};
use sce_rust_mesh::https::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};
use sce_rust_mesh::https::client::{ClientSettings, HttpsClient, HttpsClientTransport};
use sce_rust_mesh::https::conformance::{
    inspecting_proxy, tunnelling_proxy, ProxyLog, TestAuthority,
};
use sce_rust_mesh::https::server::{
    server_tls, Authenticator, HttpsServer, MailboxTransport, ServerSettings,
};
use sce_rust_mesh::https::Credential;
use sce_rust_mesh::inbound::Delivery;
use sce_rust_mesh::link::{deliver, LinkEvent};
use sce_rust_mesh::outbound::RetryPolicy;
use sce_rust_mesh::pool::{Pool, PoolConfig};
use sce_rust_mesh::reconnect::Backoff;
use sce_rust_mesh::router::{PeerConfig, Router};
use sce_rust_mesh::rpc::MESH_EVENT_PARAM;
use sce_rust_runtime::json::{self, Value};
use sce_rust_runtime::HostInvokeRequest;
use sce_rust_session_store::SqliteStore;
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

/// A server on a loopback port, under a certificate a test authority issued, with the ledger
/// behind it as a keyed pool.
struct Served {
    runtime: Runtime,
    authority: TestAuthority,
    address: SocketAddr,
    /// How many envelopes the binding handed the pool.
    received: Arc<AtomicUsize>,
    store: Arc<SqliteStore>,
    // Last, so that the store's directory is removed after everything that holds the store.
    _scratch: Scratch,
}

impl Served {
    fn start(name: &str) -> Served {
        let scratch = Scratch::new(name);
        let store = Arc::new(SqliteStore::open(scratch.store()).expect("the reference store"));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("the suite's runtime");
        let authority = TestAuthority::new("SCE server suite authority");
        let (chain, key) = authority.issue("127.0.0.1");
        let tls = server_tls(chain, key, None).expect("the server's TLS");

        // The host the binding tells what an exchange carried: it hands each envelope to the pool
        // and runs the session it names, which saves it, before the exchange is answered.
        let pool: Arc<OnceLock<HttpsPool>> = Arc::new(OnceLock::new());
        let received = Arc::new(AtomicUsize::new(0));
        let host = {
            let pool = Arc::clone(&pool);
            let received = Arc::clone(&received);
            move |events: Vec<LinkEvent>| {
                let Some(pool) = pool.get() else {
                    return;
                };
                for event in events {
                    let run = match event {
                        LinkEvent::Ready(peer) => pool.peer_ready(&peer),
                        LinkEvent::Lost(peer, _) => pool.peer_not_ready(&peer),
                        LinkEvent::Received(peer, bytes) => {
                            received.fetch_add(1, Ordering::SeqCst);
                            pool.receive(&peer, bytes).into_iter().collect()
                        }
                        // The accepting side hands back nothing it could not write: what the pool
                        // sends waits in the client's mailbox.
                        LinkEvent::Unsent(..) => Vec::new(),
                    };
                    for key in run {
                        pool.run(&key);
                    }
                }
            }
        };
        let server = HttpsServer::new(
            ServerSettings {
                max_hold: Duration::from_secs(2),
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
            Arc::clone(&store),
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
}

impl Client {
    fn start(base: String, port: HyperTransport, retry: Option<RetryPolicy>) -> Client {
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
        }
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
            assert!(Instant::now() < deadline, "waited too long for {what}");
            if let Ok(event) = self.events.recv_timeout(Duration::from_millis(20)) {
                match &event {
                    LinkEvent::Ready(_) => self.ready = true,
                    LinkEvent::Lost(..) => self.ready = false,
                    LinkEvent::Received(..) | LinkEvent::Unsent(..) => {}
                }
                deliver(&mut self.endpoint, event);
            }
            self.endpoint.tick();
            for call in self.endpoint.take_calls() {
                if let EngineCall::EndInvoke {
                    invoke_id,
                    failed,
                    data,
                    ..
                } = call
                {
                    self.ended.push(Ended {
                        invoke_id,
                        failed,
                        data,
                    });
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
    let mut client = Client::start(
        served.base(),
        adapter(&[served.authority.der()], proxy),
        None,
    );
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
    let mut client = Client::start(served.base(), adapter(&[inspector.der()], proxy), None);
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
    let mut client = Client::start(
        served.base(),
        adapter(&[inspector.der()], proxy),
        Some(retry),
    );
    client.until("the server to be ready", |client| client.ready);
    let asked = [("k1", "k1.lost")];
    client.ask(asked[0].0, asked[0].1);
    client.until("the answer", |client| !client.ended.is_empty());
    client.until("the copy sent again to reach the pool", |_| {
        served.received() >= 2
    });
    client.settle(Duration::from_millis(500));
    let ended = client.stop();

    assert!(
        log.lock().expect("the proxy's log").dropped,
        "no response was dropped"
    );
    each_answered_once_where_the_store_holds_it(&served, &asked, &ended);
}
