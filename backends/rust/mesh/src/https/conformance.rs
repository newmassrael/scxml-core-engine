// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The behaviour every adapter of the HTTPS port is held to: the Rust conformance kit of
//! SCE_MESH.md §mesh-22.7.
//!
//! The binding reaches the network through [`HttpsTransport`], so an adapter is a choice: this
//! crate's ([`super::adapter::HyperTransport`], see the test target `https_conformance`), or one a
//! program writes on its own HTTP stack. This module is what a choice has to pass: functions that
//! run a client machine through [`HttpsClient`] on whatever adapter a [`Harness`] makes, against a
//! server machine on this crate's [`HttpsServer`], and assert what the machines see.
//!
//! A client's own test target calls them from `#[test]` functions:
//!
//! ```text
//! struct MyHarness;
//!
//! impl Harness for MyHarness {
//!     type Transport = MyTransport;
//!     fn transport(&self, authorities: &[Vec<u8>], proxy: Option<&str>) -> MyTransport {
//!         MyTransport::believing(authorities).through(proxy)
//!     }
//! }
//!
//! #[test]
//! fn through_an_explicit_proxy() {
//!     conformance::through_an_explicit_proxy(&MyHarness);
//! }
//! ```
//!
//! What they hold, each with a test authority the kit makes and the harness's adapter is told to
//! believe, as an operating system is told to believe an authority its administrator installs:
//!
//! * [`an_envelope_crosses_and_its_reply_comes_back`]: a client reaches a local server, an envelope
//!   it sends reaches the server's machine once, and the reply that machine sends reaches the
//!   client's.
//! * [`a_server_under_an_unknown_authority_is_refused`]: an adapter believes no authority it was
//!   not given, and says so as [`HttpsFailure::Tls`].
//! * [`through_an_explicit_proxy`]: the adapter reaches the server through a `CONNECT` tunnel.
//! * [`through_a_proxy_that_inspects_tls`]: a proxy opens the tunnel and answers the TLS itself,
//!   under an authority of its own. An adapter that believes it reaches the server and the proxy
//!   reads the exchange; one that does not is refused as [`HttpsFailure::Tls`].
//! * [`a_response_lost_after_the_server_acted_is_carried_again`]: the proxy drops the response to
//!   the exchange that carried an envelope after the server took it in. The client sends it again,
//!   the server's machine still sees it once, and the reply it made reaches the client from the
//!   mailbox, once.
//! * [`a_held_exchange_past_its_bound_is_a_timeout`]: a server that holds an exchange longer than
//!   the request's bound is answered by the adapter as [`HttpsFailure::Timeout`], in about the
//!   bound.
//!
//! The test authority ([`TestAuthority`]) and the two proxies ([`tunnelling_proxy`],
//! [`inspecting_proxy`], with what each saw in a [`ProxyLog`]) are public as well, so that a suite
//! that puts a whole server behind the binding (`tests/server`, docs/adr/0107 decision D10) puts
//! its client behind the same proxies the kit holds every adapter to.
//!
//! Every server and proxy listens on a loopback port of its own, so the functions may run in
//! parallel. Every wait has a generous bound and the assertions are on what arrived. A failed
//! expectation panics, as a test's does.

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::net::SocketAddr;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Runtime;

use super::client::{ClientSettings, HttpsClient, HttpsClientTransport};
use super::server::{server_tls, Authenticator, HttpsServer, MailboxTransport, ServerSettings};
use super::{
    deliver, Credential, HttpsFailure, HttpsRequest, HttpsTransport, LinkEvent, ACK_HEADER,
    CONTENT_TYPE, HOLD_HEADER,
};
use crate::endpoint::{Endpoint, EngineCall, Environment, MESH_PROCESSOR_TYPE};
use crate::inbound::Delivery;
use crate::outbound::RetryPolicy;
use crate::reconnect::Backoff;
use crate::router::{PeerConfig, Router};
use sce_rust_runtime::HostSendRequest;

/// What an adapter is run through: how to make one, told which authorities to believe and which
/// proxy to use, as §mesh-22.7 asks of the kit's `Harness`.
pub trait Harness {
    /// The adapter under test.
    type Transport: HttpsTransport + 'static;

    /// An adapter that believes `authorities` (each a DER certificate) besides what the platform
    /// believes, and reaches every `https://` server through the proxy `proxy`
    /// (`http://<host>:<port>`) when one is given, and directly otherwise, whatever the platform's
    /// proxy settings say.
    fn transport(&self, authorities: &[Vec<u8>], proxy: Option<&str>) -> Self::Transport;
}

/// How long any one expectation is waited for.
const PATIENCE: Duration = Duration::from_secs(20);

/// The token the kit's client proves its name with.
const TOKEN: &str = "kit-token";

/// An authority made for one run: the certificates it issues are believed only by an adapter told
/// to believe it.
pub struct TestAuthority {
    key: rcgen::KeyPair,
    certificate: rcgen::Certificate,
}

impl TestAuthority {
    /// A fresh authority named `name`.
    pub fn new(name: &str) -> TestAuthority {
        let mut params =
            rcgen::CertificateParams::new(Vec::<String>::new()).expect("an authority's parameters");
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, name);
        params.key_usages = alloc::vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
            rcgen::KeyUsagePurpose::DigitalSignature,
        ];
        let key = rcgen::KeyPair::generate().expect("an authority's key");
        let certificate = params
            .self_signed(&key)
            .expect("an authority's certificate");
        TestAuthority { key, certificate }
    }

    /// The authority's certificate, DER: what an adapter is told to believe.
    pub fn der(&self) -> Vec<u8> {
        self.certificate.der().to_vec()
    }

    /// A server certificate for `host` (a name or an IP address) issued by this authority: the
    /// chain (each DER) and the private key (DER).
    pub fn issue(&self, host: &str) -> (Vec<Vec<u8>>, Vec<u8>) {
        let mut params = rcgen::CertificateParams::new(alloc::vec![host.to_string()])
            .expect("a server's parameters");
        params.extended_key_usages = alloc::vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
        let key = rcgen::KeyPair::generate().expect("a server's key");
        let certificate = params
            .signed_by(&key, &self.certificate, &self.key)
            .expect("a server's certificate");
        (alloc::vec![certificate.der().to_vec()], key.serialize_der())
    }
}

/// A clock that is the real one, and envelope ids that never repeat in the process.
struct RealClock {
    started: Instant,
}

impl Environment for RealClock {
    fn now_ms(&mut self) -> i64 {
        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX)
    }
    fn now_unix_ms(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0)
    }
    fn envelope_id(&mut self) -> [u8; 16] {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let mut id = [0u8; 16];
        id[8..].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        id
    }
    fn jitter_draw(&mut self) -> i64 {
        0
    }
}

fn peer_config(peer: &'static str, retry: Option<RetryPolicy>) -> PeerConfig {
    PeerConfig {
        transport: "https",
        buffer: None,
        retry,
        stamp_sequence: false,
        delivery: Delivery {
            dedup: true,
            ordered: false,
        },
        responders: Box::leak(Box::new([peer])),
        deadline_ms: None,
        reply_events: &[],
        causal: None,
    }
}

fn mesh_send(event: &str, target: &str) -> HostSendRequest {
    HostSendRequest {
        processor_type: MESH_PROCESSOR_TYPE.to_string(),
        event_name: event.to_string(),
        target: alloc::format!("#{target}"),
        send_id: alloc::format!("{event}-1"),
        event_data: r#"{"v":1}"#.to_string(),
        ..HostSendRequest::default()
    }
}

/// The names of the events `calls` raise.
fn raised_names(calls: Vec<EngineCall>) -> Vec<String> {
    calls
        .into_iter()
        .filter_map(|call| match call {
            EngineCall::Raise(event) => Some(event.name),
            EngineCall::EndInvoke { .. } => None,
        })
        .collect()
}

/// What the server machine saw: the events raised in it, and the envelopes its link handed on.
#[derive(Default)]
struct ServerLog {
    raised: Vec<String>,
    received: usize,
}

/// The server machine `cloud`: it answers every `ping` with a `pong` to `phone`.
struct ServerSide {
    endpoint: Option<Endpoint<MailboxTransport, RealClock>>,
    log: ServerLog,
}

/// A runtime with a server on a loopback port, under a certificate the kit's authority issued.
struct Bench {
    runtime: Runtime,
    server: Arc<Mutex<ServerSide>>,
    address: SocketAddr,
    authority: TestAuthority,
}

impl Bench {
    fn new() -> Bench {
        Bench::with_server(|runtime, listener, tls, server| {
            runtime.spawn(async move {
                let _ = server.serve(listener, Some(tls)).await;
            });
        })
    }

    /// A bench whose server is run by `run` on the listener, with the TLS and the server made.
    fn with_server(
        run: impl FnOnce(&Runtime, TcpListener, Arc<rustls::ServerConfig>, HttpsServer),
    ) -> Bench {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("the kit's runtime");
        let authority = TestAuthority::new("SCE kit server authority");
        let (chain, key) = authority.issue("127.0.0.1");
        let tls = server_tls(chain, key, None).expect("the server's TLS");
        let side = Arc::new(Mutex::new(ServerSide {
            endpoint: None,
            log: ServerLog::default(),
        }));
        let host = {
            let side = Arc::clone(&side);
            move |events: Vec<LinkEvent>| {
                let mut side = side.lock().expect("the server side");
                let side = &mut *side;
                let Some(endpoint) = side.endpoint.as_mut() else {
                    return;
                };
                for event in events {
                    if matches!(event, LinkEvent::Received(..)) {
                        side.log.received += 1;
                    }
                    deliver(endpoint, event);
                }
                for name in raised_names(endpoint.take_calls()) {
                    if name == "ping" {
                        endpoint.send(&mesh_send("pong", "phone"));
                    }
                    side.log.raised.push(name);
                }
            }
        };
        let server = HttpsServer::new(
            ServerSettings {
                max_hold: Duration::from_secs(2),
                // The kit judges an adapter, which may retry as fast as its proxy lets it; the
                // edge's rates are the server's own tests' to judge (docs/adr/0109, D2).
                rates: crate::peers::EdgeRates::NONE,
                ..ServerSettings::default()
            },
            Authenticator::Bearer(Arc::new(|name: &str, token: &str| {
                name == "phone" && token == TOKEN
            })),
            |name| name == "phone",
            host,
        )
        .expect("the server's mailbox incarnation");
        let mut router = Router::new("cloud", 64, 50).expect("a router");
        router.add_peer("phone", peer_config("phone", None));
        side.lock().expect("the server side").endpoint = Some(Endpoint::new(
            router,
            server.transport(),
            RealClock {
                started: Instant::now(),
            },
        ));
        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        run(&runtime, listener, tls, server);
        Bench {
            runtime,
            server: side,
            address,
            authority,
        }
    }

    fn base(&self) -> String {
        alloc::format!("https://{}", self.address)
    }

    /// The events raised in the server machine, and how many envelopes reached it.
    fn server_log(&self) -> (Vec<String>, usize) {
        let side = self.server.lock().expect("the server side");
        (side.log.raised.clone(), side.log.received)
    }
}

/// The client machine `phone`, on the harness's adapter.
struct ClientSide {
    endpoint: Endpoint<HttpsClientTransport, RealClock>,
    client: HttpsClient,
    events: mpsc::Receiver<LinkEvent>,
    raised: Vec<String>,
    ready: bool,
}

impl ClientSide {
    fn start(
        base: String,
        port: impl HttpsTransport + 'static,
        retry: Option<RetryPolicy>,
    ) -> Self {
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
                "phone",
                "cloud",
                Credential::Bearer(TOKEN.to_string()),
            )
        };
        let client = HttpsClient::start(settings, Arc::new(port), move |event| {
            let _ = tx.send(event);
        })
        .expect("a client the binding can post under");
        let mut router = Router::new("phone", 64, 50).expect("a router");
        router.add_peer("cloud", peer_config("cloud", retry));
        let endpoint = Endpoint::new(
            router,
            client.transport(),
            RealClock {
                started: Instant::now(),
            },
        );
        ClientSide {
            endpoint,
            client,
            events,
            raised: Vec::new(),
            ready: false,
        }
    }

    /// Drive the client machine (its link's events, its retries) until `done` holds of it, or
    /// panic with `what` after the kit's patience.
    fn until(&mut self, what: &str, done: impl Fn(&ClientSide) -> bool) {
        let deadline = Instant::now() + PATIENCE;
        while !done(self) {
            assert!(Instant::now() < deadline, "waited too long for {what}");
            if let Ok(event) = self.events.recv_timeout(Duration::from_millis(20)) {
                match &event {
                    LinkEvent::Ready(_) => self.ready = true,
                    LinkEvent::Lost(..) | LinkEvent::Interrupted(..) => self.ready = false,
                    LinkEvent::Received(..) | LinkEvent::Unsent(..) | LinkEvent::Restarted(_) => {}
                }
                deliver(&mut self.endpoint, event);
            }
            self.endpoint.tick();
            let names = raised_names(self.endpoint.take_calls());
            self.raised.extend(names);
        }
    }

    /// Drive the client machine for `period`: long enough for a second copy of what arrived to
    /// arrive too, were one coming.
    fn settle(&mut self, period: Duration) {
        let until = Instant::now() + period;
        self.until("the period to pass", |_| Instant::now() >= until);
    }

    fn count(&self, name: &str) -> usize {
        self.raised.iter().filter(|raised| *raised == name).count()
    }

    fn stop(self) {
        self.client.stop();
    }
}

/// The round every scenario that reaches the server makes: the client is ready, sends `ping`, and
/// gets the `pong` the server's machine answers it with; the server's machine saw `ping` once.
fn ping_pong(bench: &Bench, mut client: ClientSide) -> ClientSide {
    client.until("the server to be ready", |client| client.ready);
    client.endpoint.send(&mesh_send("ping", "cloud"));
    client.until("the reply", |client| client.count("pong") >= 1);
    client.settle(Duration::from_millis(500));
    let (server_raised, _) = bench.server_log();
    assert_eq!(
        server_raised.iter().filter(|name| *name == "ping").count(),
        1,
        "the server's machine saw {server_raised:?}"
    );
    assert_eq!(
        client.count("pong"),
        1,
        "the client's machine saw {:?}",
        client.raised
    );
    client
}

/// A client reaches a local server, an envelope it sends reaches the server's machine once, and
/// the reply that machine sends reaches the client's.
pub fn an_envelope_crosses_and_its_reply_comes_back<H: Harness>(harness: &H) {
    let bench = Bench::new();
    let port = harness.transport(&[bench.authority.der()], None);
    let client = ping_pong(&bench, ClientSide::start(bench.base(), port, None));
    assert!(
        !client
            .raised
            .iter()
            .any(|name| name == "error.communication"),
        "{:?}",
        client.raised
    );
    client.stop();
}

/// A poll of the binding, as the port carries it.
fn poll(url: String) -> HttpsRequest {
    HttpsRequest {
        url,
        headers: alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), "0".to_string()),
            (HOLD_HEADER.to_string(), "0".to_string()),
            (
                "Authorization".to_string(),
                alloc::format!("Bearer {TOKEN}")
            ),
        ],
        body: Vec::new(),
        timeout: Duration::from_secs(10),
    }
}

/// An adapter believes no authority it was not given: a server under the kit's authority is
/// refused in the handshake, as [`HttpsFailure::Tls`].
pub fn a_server_under_an_unknown_authority_is_refused<H: Harness>(harness: &H) {
    let bench = Bench::new();
    let stranger = TestAuthority::new("SCE kit stranger");
    let port = harness.transport(&[stranger.der()], None);
    let refused = port.exchange(poll(alloc::format!("{}/sce-mesh/1/phone", bench.base())));
    assert_eq!(refused, Err(HttpsFailure::Tls));
}

/// What a proxy saw.
#[derive(Default)]
pub struct ProxyLog {
    /// The target of every `CONNECT`.
    pub tunnels: Vec<String>,
    /// What an inspecting proxy read from its clients, in the clear.
    pub plaintext: Vec<u8>,
    /// Whether it dropped a response.
    pub dropped: bool,
}

/// Read a `CONNECT` request's head from `stream`, a byte at a time, and return its target.
async fn read_connect(stream: &mut TcpStream) -> Option<String> {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 8192 {
            return None;
        }
        head.push(stream.read_u8().await.ok()?);
    }
    let line = String::from_utf8(head).ok()?;
    let mut words = line.split(' ');
    (words.next()? == "CONNECT").then_some(())?;
    words.next().map(str::to_string)
}

/// A proxy that tunnels: it answers `CONNECT` and copies bytes both ways. It listens on a loopback
/// port of its own, which it answers, and runs on `runtime` until the runtime ends.
pub fn tunnelling_proxy(runtime: &Runtime, log: Arc<Mutex<ProxyLog>>) -> SocketAddr {
    let listener = runtime
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("a loopback port");
    let address = listener.local_addr().expect("its address");
    runtime.spawn(async move {
        while let Ok((mut client, _)) = listener.accept().await {
            let log = Arc::clone(&log);
            tokio::spawn(async move {
                let Some(target) = read_connect(&mut client).await else {
                    return;
                };
                log.lock()
                    .expect("the proxy log")
                    .tunnels
                    .push(target.clone());
                let Ok(mut server) = TcpStream::connect(target.as_str()).await else {
                    let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\n").await;
                    return;
                };
                if client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .is_err()
                {
                    return;
                }
                let _ = tokio::io::copy_bidirectional(&mut client, &mut server).await;
            });
        }
    });
    address
}

/// Whether `bytes`, the head of a request read in the clear, declares a body that is not empty.
fn declares_a_body(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    text.split("\r\n").any(|line| {
        line.strip_prefix("content-length:")
            .and_then(|length| length.trim().parse::<u64>().ok())
            .is_some_and(|length| length > 0)
    })
}

/// A proxy that inspects TLS: it answers `CONNECT`, then answers the client's TLS itself with a
/// certificate for the target that `authority` issues, opens its own TLS to the server believing
/// `upstream` (DER), and copies what it reads in the clear both ways, keeping what the client
/// wrote. With `drop_one`, it drops the first response to a request that carried a body: the
/// server has taken the request in by then, and the client never sees what it answered. It listens
/// on a loopback port of its own, which it answers, and runs on `runtime` until the runtime ends.
pub fn inspecting_proxy(
    runtime: &Runtime,
    authority: Arc<TestAuthority>,
    upstream: Vec<u8>,
    drop_one: bool,
    log: Arc<Mutex<ProxyLog>>,
) -> SocketAddr {
    let listener = runtime
        .block_on(TcpListener::bind("127.0.0.1:0"))
        .expect("a loopback port");
    let address = listener.local_addr().expect("its address");
    let upstream_tls = crate::tls::client_config(&crate::tls::Roots::Only(alloc::vec![upstream]))
        .expect("the proxy's TLS to the server");
    let dropped = Arc::new(AtomicBool::new(false));
    runtime.spawn(async move {
        while let Ok((mut client, _)) = listener.accept().await {
            let log = Arc::clone(&log);
            let authority = Arc::clone(&authority);
            let upstream_tls = Arc::clone(&upstream_tls);
            let dropped = Arc::clone(&dropped);
            tokio::spawn(async move {
                let Some(target) = read_connect(&mut client).await else {
                    return;
                };
                log.lock()
                    .expect("the proxy log")
                    .tunnels
                    .push(target.clone());
                let host = target
                    .rsplit_once(':')
                    .map(|(host, _)| host.to_string())
                    .unwrap_or_else(|| target.clone());
                if client
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .await
                    .is_err()
                {
                    return;
                }
                // HTTP/1.1 only, on both legs, so that what it reads is one request after another.
                let (chain, key) = authority.issue(&host);
                let Ok(made) = server_tls(chain, key, None) else {
                    return;
                };
                let mut facing = (*made).clone();
                facing.alpn_protocols = alloc::vec![b"http/1.1".to_vec()];
                let Ok(client) = tokio_rustls::TlsAcceptor::from(Arc::new(facing))
                    .accept(client)
                    .await
                else {
                    return;
                };
                let Ok(tcp) = TcpStream::connect(target.as_str()).await else {
                    return;
                };
                let Ok(name) = rustls::pki_types::ServerName::try_from(host) else {
                    return;
                };
                let Ok(server) = tokio_rustls::TlsConnector::from(upstream_tls)
                    .connect(name, tcp)
                    .await
                else {
                    return;
                };
                relay(client, server, drop_one, dropped, log).await;
            });
        }
    });
    address
}

/// Copy what `client` and `server` write to each other until either ends, keeping what the client
/// wrote, and with `drop_one` closing both instead of passing on the first response to a request
/// that carried a body (`dropped` says whether some connection did already).
async fn relay<C, S>(
    client: C,
    server: S,
    drop_one: bool,
    dropped: Arc<AtomicBool>,
    log: Arc<Mutex<ProxyLog>>,
) where
    C: AsyncRead + AsyncWrite + Unpin,
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (mut client_read, mut client_write) = tokio::io::split(client);
    let (mut server_read, mut server_write) = tokio::io::split(server);
    // The request read since the last response began: what tells a request with a body.
    let pending = Mutex::new(Vec::new());
    let up = async {
        let mut buffer = [0u8; 8192];
        loop {
            let Ok(read) = client_read.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            let chunk = &buffer[..read];
            pending.lock().expect("pending").extend_from_slice(chunk);
            {
                let mut log = log.lock().expect("the proxy log");
                if log.plaintext.len() < 1 << 20 {
                    log.plaintext.extend_from_slice(chunk);
                }
            }
            if server_write.write_all(chunk).await.is_err() {
                return;
            }
        }
    };
    let down = async {
        let mut buffer = [0u8; 8192];
        loop {
            let Ok(read) = server_read.read(&mut buffer).await else {
                return;
            };
            if read == 0 {
                return;
            }
            let carried_a_body = {
                let mut pending = pending.lock().expect("pending");
                let carried = declares_a_body(&pending);
                pending.clear();
                carried
            };
            if drop_one && carried_a_body && !dropped.swap(true, Ordering::AcqRel) {
                log.lock().expect("the proxy log").dropped = true;
                return;
            }
            if client_write.write_all(&buffer[..read]).await.is_err() {
                return;
            }
        }
    };
    tokio::select! {
        _ = up => {}
        _ = down => {}
    }
}

/// The adapter reaches the server through a `CONNECT` tunnel, and the exchange is the same.
pub fn through_an_explicit_proxy<H: Harness>(harness: &H) {
    let bench = Bench::new();
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = tunnelling_proxy(&bench.runtime, Arc::clone(&log));
    let port = harness.transport(
        &[bench.authority.der()],
        Some(&alloc::format!("http://{proxy}")),
    );
    let client = ping_pong(&bench, ClientSide::start(bench.base(), port, None));
    client.stop();
    let server = bench.address.to_string();
    let log = log.lock().expect("the proxy log");
    let tunnels = &log.tunnels;
    assert!(!tunnels.is_empty(), "nothing went through the proxy");
    assert!(
        tunnels.iter().all(|target| *target == server),
        "{tunnels:?}"
    );
}

/// A proxy that inspects TLS is honoured by an adapter that believes its authority (as a machine
/// whose administrator installed it does), and refused as [`HttpsFailure::Tls`] by one that does
/// not.
pub fn through_a_proxy_that_inspects_tls<H: Harness>(harness: &H) {
    let bench = Bench::new();
    let inspector = Arc::new(TestAuthority::new("SCE kit inspecting authority"));
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = inspecting_proxy(
        &bench.runtime,
        Arc::clone(&inspector),
        bench.authority.der(),
        false,
        Arc::clone(&log),
    );
    let proxy_url = alloc::format!("http://{proxy}");

    let refusing = harness.transport(&[bench.authority.der()], Some(&proxy_url));
    let refused = refusing.exchange(poll(alloc::format!("{}/sce-mesh/1/phone", bench.base())));
    assert_eq!(refused, Err(HttpsFailure::Tls));

    let believing = harness.transport(&[inspector.der()], Some(&proxy_url));
    let client = ping_pong(&bench, ClientSide::start(bench.base(), believing, None));
    client.stop();
    let plaintext =
        String::from_utf8_lossy(&log.lock().expect("the proxy log").plaintext).to_ascii_lowercase();
    assert!(
        plaintext.contains("post /sce-mesh/1/phone http/1.1"),
        "the proxy read no exchange in the clear"
    );
}

/// The proxy drops the response to the exchange that carried `ping`, after the server took it in
/// and its machine answered. The client is told the exchange failed and sends `ping` again under
/// its retry policy; the server's duplicate window drops the second copy, so its machine sees
/// `ping` once; and the `pong` it made waits in the mailbox until an exchange carries it, so the
/// client's machine sees it once.
pub fn a_response_lost_after_the_server_acted_is_carried_again<H: Harness>(harness: &H) {
    let bench = Bench::new();
    let inspector = Arc::new(TestAuthority::new("SCE kit dropping authority"));
    let log = Arc::new(Mutex::new(ProxyLog::default()));
    let proxy = inspecting_proxy(
        &bench.runtime,
        Arc::clone(&inspector),
        bench.authority.der(),
        true,
        Arc::clone(&log),
    );
    let port = harness.transport(&[inspector.der()], Some(&alloc::format!("http://{proxy}")));
    let retry = RetryPolicy {
        max_retries: 5,
        initial_backoff_ms: 50,
        backoff_multiplier: 2.0,
        max_backoff_ms: 1000,
        jitter_pct: 0,
    };
    let mut client = ClientSide::start(bench.base(), port, Some(retry));
    client.until("the server to be ready", |client| client.ready);
    client.endpoint.send(&mesh_send("ping", "cloud"));
    client.until("the reply", |client| client.count("pong") >= 1);
    client.until("the copy sent again to reach the server", |_| {
        bench.server_log().1 >= 2
    });
    client.settle(Duration::from_millis(500));
    // What the client's machine saw, read before stopping it, which takes it.
    let pongs = client.count("pong");
    let client_raised = client.raised.clone();
    client.stop();

    assert!(
        log.lock().expect("the proxy log").dropped,
        "no response was dropped"
    );
    let (server_raised, received) = bench.server_log();
    assert_eq!(
        server_raised.iter().filter(|name| *name == "ping").count(),
        1,
        "the server's machine saw {server_raised:?} from {received} envelopes"
    );
    assert_eq!(pongs, 1, "the client's machine saw {client_raised:?}");
}

/// A server that holds an exchange longer than the request's bound is answered by the adapter as
/// [`HttpsFailure::Timeout`], in about the bound and not in the server's time.
pub fn a_held_exchange_past_its_bound_is_a_timeout<H: Harness>(harness: &H) {
    // A server that takes the TLS and the request and never answers.
    let bench = Bench::with_server(|runtime, listener, tls, _server| {
        runtime.spawn(async move {
            let acceptor = tokio_rustls::TlsAcceptor::from(tls);
            while let Ok((stream, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        return;
                    };
                    let mut buffer = [0u8; 1024];
                    while matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {}
                });
            }
        });
    });
    let port = harness.transport(&[bench.authority.der()], None);
    let mut held = poll(alloc::format!("{}/sce-mesh/1/phone", bench.base()));
    held.timeout = Duration::from_millis(500);
    let began = Instant::now();
    let late = port.exchange(held);
    let took = began.elapsed();
    assert_eq!(late, Err(HttpsFailure::Timeout));
    assert!(
        took >= Duration::from_millis(400),
        "answered early, in {took:?}"
    );
    assert!(took < Duration::from_secs(5), "answered late, in {took:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_declares_a_body_by_a_length_above_zero() {
        assert!(declares_a_body(
            b"POST /sce-mesh/1/phone HTTP/1.1\r\ncontent-length: 37\r\n\r\n"
        ));
        assert!(declares_a_body(
            b"POST / HTTP/1.1\r\nContent-Length:  2\r\n\r\n"
        ));
        assert!(!declares_a_body(
            b"POST /sce-mesh/1/phone HTTP/1.1\r\ncontent-length: 0\r\n\r\n"
        ));
        assert!(!declares_a_body(b"POST / HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn a_test_authority_issues_what_it_alone_vouches_for() {
        let authority = TestAuthority::new("one");
        let (chain, key) = authority.issue("127.0.0.1");
        assert_eq!(chain.len(), 1);
        assert!(server_tls(chain, key, None).is_ok());
        assert_ne!(authority.der(), TestAuthority::new("two").der());
    }
}
