// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The accepting side of the HTTPS binding (SCE_MESH.md section 22): the server machine's end,
//! where each client's envelopes wait in a mailbox until the client's next exchange carries them.
//!
//! The server of §mesh-22.8: hyper on tokio, HTTP/1.1 and HTTP/2 on one listener, TLS from
//! [`server_tls`], the mailbox as the endpoint's transport. For each exchange the server reads
//! the client's name from the path, checks the authenticator, drops what the client
//! acknowledged, hands the core what the request carried and waits until the macrosteps it
//! caused have run ([`ServerHost::deliver`]), then answers with what the mailbox holds, holding
//! the exchange while it holds nothing (§mesh-22.3).
//!
//! What the machine sends a client reaches the mailbox through [`MailboxTransport`], the
//! transport the host builds its endpoint on. A full mailbox refuses the envelope as a full queue
//! ([`TransportFailure::queue_full`]), which the core raises as SCE_MESH.md section 16.7 row 9.
//!
//! The mailboxes of one server number in one incarnation, drawn when the server starts with its
//! mailboxes empty and named on every `200` (`Sce-Mesh-Epoch`), so that a client tells a mailbox
//! that numbers anew from one it knows. A host whose endpoint checkpoints (SCE_MESH.md section
//! 10.11) answers an exchange that carried envelopes only once a durable checkpoint holds them
//! ([`ServerHost::deliver`], [`Durability`], [`HttpsServer::durable`]); a keyed pool reports it
//! per exchange from the receipts of the envelopes it was handed ([`crate::pool::Receipt`]).

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::Incoming;
use hyper::header::{
    HeaderValue, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE as CONTENT_TYPE_NAME,
};
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::Notify;

use super::{
    decode_install_secret, draw_epoch, install_digest, is_epoch, is_path_segment, split_sequence,
    LinkEvent, ACK_HEADER, BEARER_SCHEME, CONTENT_TYPE, EPOCH_HEADER, FIRST_HEADER, HOLD_HEADER,
    INSTALL_SCHEME, PATH_ROOT, VERSION,
};
use crate::endpoint::{Transport, TransportFailure};
use crate::peers::{HttpsAuthenticator, Machine, PeerLink};

/// The bounds a server keeps: deploy.yaml's keys of the accepting side, and two of the host's own
/// for shedding load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSettings {
    /// The path the binding is served under, before `/sce-mesh/`: empty, or `/` followed by
    /// segments, without a trailing `/`.
    pub base_path: String,
    /// deploy.yaml's `max_hold_ms`: the longest an exchange is held while the mailbox is empty.
    pub max_hold: Duration,
    /// deploy.yaml's `max_body_bytes`: the largest request body taken in.
    pub max_body_bytes: usize,
    /// deploy.yaml's `absence_ms`: how long a client may go without an exchange and stay ready.
    pub absence: Duration,
    /// deploy.yaml's `mailbox`: how many envelopes wait for one client at most.
    pub mailbox: usize,
    /// The most exchanges in flight at once; one more is answered `503`.
    pub max_exchanges: usize,
    /// The most exchanges in flight for one client; one more is answered `429`.
    pub max_exchanges_per_client: usize,
    /// What a `429` or `503` asks the client to wait (`Retry-After`, in whole seconds).
    pub retry_after: Duration,
}

impl Default for ServerSettings {
    fn default() -> Self {
        ServerSettings {
            base_path: String::new(),
            max_hold: Duration::from_millis(u64::from(super::DEFAULT_MAX_HOLD_MS)),
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES as usize,
            absence: Duration::from_millis(u64::from(super::DEFAULT_ABSENCE_MS)),
            mailbox: super::DEFAULT_MAILBOX as usize,
            max_exchanges: 10_000,
            // The client's poller and its sender, and one more for an exchange the client gave up
            // on that the server has not yet seen end.
            max_exchanges_per_client: 3,
            retry_after: Duration::from_secs(1),
        }
    }
}

/// What a machine's peer table says of one address it listens on for the HTTPS binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listening {
    /// The bounds: of each, the smallest any binding on the address declares, so that no binding
    /// is given more than it declared.
    pub settings: ServerSettings,
    /// The authenticator every binding on the address names.
    pub authenticator: HttpsAuthenticator,
    /// The peers accepted on the address: the names a server built for it binds.
    pub peers: Vec<&'static str>,
}

impl Listening {
    /// What `machine` accepts over HTTPS on `listen` ([`PeerLink::HttpsAccept`]). `None` when no
    /// peer is accepted there, or when the bindings there name different authenticators, which
    /// one server cannot check by one rule.
    pub fn on(machine: &Machine, listen: &str) -> Option<Listening> {
        let mut found: Option<Listening> = None;
        for peer in machine.peers {
            let PeerLink::HttpsAccept {
                listen: at,
                authenticator,
                max_hold_ms,
                max_body_bytes,
                absence_ms,
                mailbox,
            } = peer.link
            else {
                continue;
            };
            if at != listen {
                continue;
            }
            let declared = ServerSettings {
                max_hold: Duration::from_millis(u64::from(max_hold_ms)),
                max_body_bytes: usize::try_from(max_body_bytes).unwrap_or(usize::MAX),
                absence: Duration::from_millis(u64::from(absence_ms)),
                mailbox: mailbox as usize,
                ..ServerSettings::default()
            };
            if let Some(held) = found.as_mut() {
                if held.authenticator != authenticator {
                    return None;
                }
                let settings = &mut held.settings;
                settings.max_hold = settings.max_hold.min(declared.max_hold);
                settings.max_body_bytes = settings.max_body_bytes.min(declared.max_body_bytes);
                settings.absence = settings.absence.min(declared.absence);
                settings.mailbox = settings.mailbox.min(declared.mailbox);
                held.peers.push(peer.name);
            } else {
                found = Some(Listening {
                    settings: declared,
                    authenticator,
                    peers: alloc::vec![peer.name],
                });
            }
        }
        found
    }
}

/// A host's check of a bearer token: `(client name, token)`, true when the token proves the name.
pub type BearerCheck = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// A host's check of a client certificate: `(client name, the certificate's DER)`, true when the
/// certificate (which TLS already verified against the deployment's client authorities) names
/// the client.
pub type CertificateCheck = Arc<dyn Fn(&str, &[u8]) -> bool + Send + Sync>;

/// Who may claim a name (§mesh-22.5). A binding with none is refused at deploy validation, so a
/// server is never built without one.
#[derive(Clone)]
pub enum Authenticator {
    /// `Authorization: Bearer <token>`, verified by the host.
    Bearer(BearerCheck),
    /// `Authorization: SCE-Install <secret>`: the first secret a name brings is pinned, by its
    /// SHA-256, at the first exchange answered `200`, and no other is taken for it after.
    Install(Arc<dyn PinStore>),
    /// A TLS client certificate, required by the TLS configuration ([`server_tls`] with client
    /// authorities) and checked against the name by the host.
    Mtls(CertificateCheck),
}

/// Where a server keeps the installation pins: name to the SHA-256 of its secret. A server that
/// keeps them in memory ([`MemoryPins`]) forgets them when it stops, and the next secret a name
/// brings is pinned again; a host that keeps them durably implements this on its store.
pub trait PinStore: Send + Sync {
    /// The digest `name` is pinned to.
    fn pinned(&self, name: &str) -> Option<[u8; 32]>;
    /// Pin `name` to `digest` unless it is pinned already, in one step; true when `name` is now
    /// pinned to `digest`, whether by this call or before it.
    fn pin(&self, name: &str, digest: [u8; 32]) -> bool;
}

/// Pins held in memory.
#[derive(Default)]
pub struct MemoryPins(Mutex<HashMap<String, [u8; 32]>>);

impl PinStore for MemoryPins {
    fn pinned(&self, name: &str) -> Option<[u8; 32]> {
        self.0.lock().expect(HELD).get(name).copied()
    }

    fn pin(&self, name: &str, digest: [u8; 32]) -> bool {
        let mut pins = self.0.lock().expect(HELD);
        let held = pins.entry(name.to_string()).or_insert(digest);
        same_digest(held, &digest)
    }
}

/// Two digests compared in time that does not depend on where they differ.
fn same_digest(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// When a `200` may acknowledge the envelopes an exchange carried, as the host that took them in
/// says (SCE_MESH.md section 10.11: nothing is acknowledged before a checkpoint that holds it is
/// durable). The server waits as it says, and answers nothing the host has not vouched for: the
/// three answers of §mesh-22.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    /// Now, once the macrosteps have run: the host's endpoint does not checkpoint, or a durable
    /// checkpoint already holds every envelope the exchange carried (a keyed pool's session, whose
    /// checkpoint is written in the run, reports this from the envelopes' receipts,
    /// [`crate::pool::Receipt`]).
    Now,
    /// Once the host reports durable a checkpoint above this number ([`HttpsServer::durable`]):
    /// the endpoint's `sealed_through()` read just before the first envelope was admitted, under
    /// the exclusion the host admits and seals under, so that every envelope admitted after it is
    /// held by a checkpoint above it. For a host whose checkpoints become durable after the call
    /// returns. One not durable within `max_hold` is answered as [`Durability::NotDurable`] is.
    After(u64),
    /// Not by this exchange: the checkpoint that would hold the envelopes was not made durable
    /// (its write failed, or a fence refused it), or the session that took them in was dropped
    /// first. The exchange is answered `503` with `Retry-After`, as load shed (§mesh-22.4): the
    /// client carries the envelopes again, the section 10.5 window drops what was taken in, and a
    /// later exchange is answered once a checkpoint holds them.
    NotDurable,
}

/// What the server asks of its host (§mesh-22.8): tell the endpoint what the binding observed,
/// run the macrosteps it caused, and return once they have run. What the machine sent meanwhile
/// is then in the mailboxes, which is what lets an exchange answer with the replies its own
/// request caused.
///
/// Called on a thread of tokio's blocking pool, one call at a time, in the order the binding
/// observed the events. A closure that takes the events is a host whose endpoint does not
/// checkpoint.
pub trait ServerHost: Send + Sync + 'static {
    /// Hand `events` to the endpoint, in order ([`super::deliver`]), and run the machine; answer
    /// when a `200` may acknowledge the envelopes among them ([`Durability`]). The server reads
    /// the answer only for an exchange that carried envelopes: a poll waits for no checkpoint.
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability;
}

impl<F: Fn(Vec<LinkEvent>) + Send + Sync + 'static> ServerHost for F {
    fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
        self(events);
        Durability::Now
    }
}

const HELD: &str = "the server's state is never left mid-update: nothing in it panics while held";

/// One client's mailbox and presence.
struct Client {
    /// The envelopes not yet acknowledged, by number, oldest first; the numbers are consecutive.
    entries: VecDeque<(u64, Vec<u8>)>,
    /// The number the next envelope gets; the first is 1.
    next: u64,
    ready: bool,
    in_flight: usize,
    last_seen: Instant,
    arrived: Arc<Notify>,
}

impl Client {
    fn new() -> Self {
        Client {
            entries: VecDeque::new(),
            next: 1,
            ready: false,
            in_flight: 0,
            last_seen: Instant::now(),
            arrived: Arc::new(Notify::new()),
        }
    }

    /// Drop what `ack` acknowledges (§mesh-22.3), when `epoch`, the incarnation the request says
    /// its acknowledgement counts in, is `current`, the incarnation this mailbox numbers in. An
    /// acknowledgement of another incarnation, or of none, counts in a numbering this mailbox
    /// does not hold, and drops nothing.
    fn acknowledge(&mut self, ack: u64, epoch: Option<&str>, current: &str) {
        if epoch != Some(current) {
            return;
        }
        while self
            .entries
            .front()
            .is_some_and(|(number, _)| *number <= ack)
        {
            self.entries.pop_front();
        }
    }
}

/// Every client's mailbox, shared by the server and the transport the endpoint sends through.
struct Mailboxes {
    capacity: usize,
    /// The incarnation every mailbox here numbers in (§mesh-22.3): drawn when the server started
    /// with its mailboxes empty, and named on every `200`.
    epoch: String,
    clients: Mutex<HashMap<String, Client>>,
}

impl Mailboxes {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Client>> {
        self.clients.lock().expect(HELD)
    }
}

/// The core's side of the server: what the machine sends a client waits in its mailbox, numbered
/// in arrival order and bounded by `mailbox` (§mesh-22.3).
#[derive(Clone)]
pub struct MailboxTransport {
    mailboxes: Arc<Mailboxes>,
}

impl Transport for MailboxTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        let mut clients = self.mailboxes.lock();
        let client = clients.entry(peer.to_string()).or_insert_with(Client::new);
        if client.entries.len() >= self.mailboxes.capacity {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!("the mailbox of {peer} is full")),
                queue_full: Some(u32::try_from(client.entries.len()).unwrap_or(u32::MAX)),
            });
        }
        client.entries.push_back((client.next, bytes.to_vec()));
        client.next += 1;
        client.arrived.notify_waiters();
        Ok(())
    }
}

struct Shared {
    settings: ServerSettings,
    authenticator: Authenticator,
    is_bound: Box<dyn Fn(&str) -> bool + Send + Sync>,
    host: Arc<dyn ServerHost>,
    mailboxes: Arc<Mailboxes>,
    /// Held while readiness is decided and the host is told, so that what the host is told about
    /// a client arrives in the order it was decided.
    telling: tokio::sync::Mutex<()>,
    exchanges: AtomicUsize,
    /// The last checkpoint the host reported durable ([`HttpsServer::durable`]), and the signal
    /// an exchange waiting for one wakes on.
    durable_through: AtomicU64,
    durable_reported: Notify,
}

/// The server: its mailboxes, its authenticator, and the host it tells.
#[derive(Clone)]
pub struct HttpsServer {
    shared: Arc<Shared>,
}

impl HttpsServer {
    /// A server with `settings`, letting a client in by `authenticator` when `is_bound` names it
    /// (the machine has a binding to it), and telling `host` what happens. Its mailboxes start
    /// empty, in an incarnation drawn from the operating system's random source; refused when the
    /// source could not be read.
    pub fn new(
        settings: ServerSettings,
        authenticator: Authenticator,
        is_bound: impl Fn(&str) -> bool + Send + Sync + 'static,
        host: impl ServerHost,
    ) -> Result<Self, crate::tls::Error> {
        let mailboxes = Arc::new(Mailboxes {
            capacity: settings.mailbox,
            epoch: draw_epoch()?,
            clients: Mutex::new(HashMap::new()),
        });
        Ok(HttpsServer {
            shared: Arc::new(Shared {
                settings,
                authenticator,
                is_bound: Box::new(is_bound),
                host: Arc::new(host),
                mailboxes,
                telling: tokio::sync::Mutex::new(()),
                exchanges: AtomicUsize::new(0),
                durable_through: AtomicU64::new(0),
                durable_reported: Notify::new(),
            }),
        })
    }

    /// The transport the host builds its endpoint on.
    pub fn transport(&self) -> MailboxTransport {
        MailboxTransport {
            mailboxes: Arc::clone(&self.shared.mailboxes),
        }
    }

    /// The incarnation the mailboxes number in (`Sce-Mesh-Epoch`, §mesh-22.3).
    pub fn epoch(&self) -> &str {
        &self.shared.mailboxes.epoch
    }

    /// The host reported the checkpoint `through`, and every one before it, durable to its
    /// endpoint (`Endpoint::durable`, SCE_MESH.md section 10.11): an exchange whose envelopes such
    /// a checkpoint holds is answered now. A host whose endpoint checkpoints calls this after each
    /// report, from any thread.
    pub fn durable(&self, through: u64) {
        self.shared
            .durable_through
            .fetch_max(through, Ordering::AcqRel);
        self.shared.durable_reported.notify_waiters();
    }

    /// Accept on `listener` until it fails, with TLS from `tls` (or none, for a loopback test),
    /// serving each connection on a task of its own, and watch for absent clients meanwhile.
    pub async fn serve(
        &self,
        listener: TcpListener,
        tls: Option<Arc<rustls::ServerConfig>>,
    ) -> std::io::Error {
        let watcher = AbortOnDrop(tokio::spawn({
            let server = self.clone();
            async move { server.watch_absence().await }
        }));
        let acceptor = tls.map(tokio_rustls::TlsAcceptor::from);
        let error = loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(error) => break error,
            };
            let server = self.clone();
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                match acceptor {
                    Some(acceptor) => {
                        let Ok(stream) = acceptor.accept(stream).await else {
                            return;
                        };
                        let certificate = stream
                            .get_ref()
                            .1
                            .peer_certificates()
                            .and_then(|chain| chain.first())
                            .map(|certificate| certificate.as_ref().to_vec());
                        server.serve_connection(stream, certificate).await;
                    }
                    None => server.serve_connection(stream, None).await,
                }
            });
        };
        drop(watcher);
        error
    }

    /// Serve the exchanges of one connection the host accepted (and, for TLS, already
    /// handshook), whose client presented `certificate` (DER), until it closes. A host that
    /// accepts itself runs [`watch_absence`](Self::watch_absence) beside it.
    pub async fn serve_connection<S>(&self, stream: S, certificate: Option<Vec<u8>>)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let shared = Arc::clone(&self.shared);
        let certificate = certificate.map(Arc::new);
        let service = hyper::service::service_fn(move |request| {
            let shared = Arc::clone(&shared);
            let certificate = certificate.clone();
            async move { Ok::<_, Infallible>(exchange(shared, request, certificate).await) }
        });
        let builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
        let _ = builder
            .serve_connection(TokioIo::new(stream), service)
            .await;
    }

    /// Tell the host, once per edge, of every ready client whose last exchange ended longer ago
    /// than `absence` and that has none in flight: SCE_MESH.md section 16.7 row 1. Runs until
    /// dropped. This is the server's half of readiness in §mesh-22.4: an exchange in flight
    /// counts as presence, and the mailbox stays.
    pub async fn watch_absence(&self) {
        let absence = self.shared.settings.absence;
        let every = (absence / 4).clamp(Duration::from_millis(10), Duration::from_secs(1));
        let mut ticker = tokio::time::interval(every);
        loop {
            ticker.tick().await;
            let _telling = self.shared.telling.lock().await;
            let lost: Vec<LinkEvent> = {
                let mut clients = self.shared.mailboxes.lock();
                clients
                    .iter_mut()
                    .filter(|(_, client)| {
                        client.ready
                            && client.in_flight == 0
                            && client.last_seen.elapsed() >= absence
                    })
                    .map(|(name, client)| {
                        client.ready = false;
                        LinkEvent::Lost(
                            name.clone(),
                            alloc::format!("no exchange for more than {} ms", absence.as_millis()),
                        )
                    })
                    .collect()
            };
            if !lost.is_empty() {
                tell(&self.shared, lost).await;
            }
        }
    }
}

/// Aborts a task when dropped, so that what [`HttpsServer::serve`] started ends with it.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Hand `events` to the host on the blocking pool, and wait until it has run them. Answers what
/// the host's [`ServerHost::deliver`] said of the envelopes among them.
async fn tell(shared: &Arc<Shared>, events: Vec<LinkEvent>) -> Durability {
    let host = Arc::clone(&shared.host);
    // A host that panicked has left its endpoint in a state nobody can read, so nothing says
    // whether the envelopes it was handed were taken, let alone made durable (SCE_MESH.md
    // §10.11): the exchange is shed as one the host could not make durable, the sender carries
    // the envelopes again, and the mailbox keeps what it holds for the next exchange. The panic
    // is the host's to see in its own log.
    tokio::task::spawn_blocking(move || host.deliver(events))
        .await
        .unwrap_or(Durability::NotDurable)
}

/// Wait until the host reported durable a checkpoint above `sealed`, at most `bound`. Whether it
/// did.
async fn durable_past(shared: &Shared, sealed: u64, bound: Duration) -> bool {
    let until = tokio::time::Instant::now() + bound;
    loop {
        let reported = shared.durable_reported.notified();
        tokio::pin!(reported);
        reported.as_mut().enable();
        if shared.durable_through.load(Ordering::Acquire) > sealed {
            return true;
        }
        if tokio::time::Instant::now() >= until {
            return false;
        }
        tokio::select! {
            _ = &mut reported => {}
            _ = tokio::time::sleep_until(until) => {}
        }
    }
}

/// Counts an exchange in flight, server-wide, for as long as it lives.
struct Counted<'a>(&'a AtomicUsize);

impl Drop for Counted<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Marks a client's exchange in flight for as long as it lives, and its end as the client's last
/// sight.
struct InFlight {
    mailboxes: Arc<Mailboxes>,
    name: String,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        let mut clients = self.mailboxes.lock();
        if let Some(client) = clients.get_mut(&self.name) {
            client.in_flight = client.in_flight.saturating_sub(1);
            client.last_seen = Instant::now();
        }
    }
}

fn answer(status: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::new()));
    *response.status_mut() = status;
    response
}

/// Load shed (§mesh-22.4): `429` or `503` with the `Retry-After` the client waits at least before
/// it sends what the request carried again.
fn shed(status: StatusCode, wait: Duration) -> Response<Full<Bytes>> {
    let mut response = answer(status);
    let seconds = wait.as_secs().max(1).to_string();
    if let Ok(value) = HeaderValue::from_str(&seconds) {
        response.headers_mut().insert("Retry-After", value);
    }
    response
}

fn unauthorized(authenticator: &Authenticator) -> Response<Full<Bytes>> {
    let mut response = answer(StatusCode::UNAUTHORIZED);
    // RFC 9110 section 15.5.2: a 401 names the scheme it asks for. A certificate is asked for by
    // TLS, not by a header, and is named here by the binding's own word.
    let scheme = match authenticator {
        Authenticator::Bearer(_) => BEARER_SCHEME,
        Authenticator::Install(_) => INSTALL_SCHEME,
        Authenticator::Mtls(_) => "SCE-Certificate",
    };
    response
        .headers_mut()
        .insert("WWW-Authenticate", HeaderValue::from_static(scheme));
    response
}

/// The client name a path posts under, when it is `<base>/sce-mesh/<version>/<name>` with the
/// version this server speaks; `None` is a `404`. The path is the only place the server learns
/// the client's name from (§mesh-22.1).
fn client_of<'a>(base_path: &str, path: &'a str) -> Option<&'a str> {
    let rest = path.strip_prefix(base_path)?.strip_prefix(PATH_ROOT)?;
    let (version, name) = rest.split_once('/')?;
    (version == VERSION && is_path_segment(name)).then_some(name)
}

/// A request header read as a number: absent is 0, anything but digits is `None` (a `400`, as
/// §mesh-22.1 answers a binding header that is not a number).
fn number_header(request: &Request<Incoming>, name: &str) -> Option<u64> {
    match request.headers().get(name) {
        None => Some(0),
        Some(value) => value.to_str().ok()?.trim().parse().ok(),
    }
}

/// The incarnation a request's acknowledgement counts in (`Sce-Mesh-Epoch`): `Some(None)` when
/// the request names none, `None` (a `400`) when the header is not an incarnation.
fn epoch_header(request: &Request<Incoming>) -> Option<Option<String>> {
    match request.headers().get(EPOCH_HEADER) {
        None => Some(None),
        Some(value) => {
            let text = value.to_str().ok()?.trim();
            is_epoch(text).then(|| Some(text.to_string()))
        }
    }
}

/// The credential text after `scheme` in the `Authorization` header.
fn credential<'a>(request: &'a Request<Incoming>, scheme: &str) -> Option<&'a str> {
    let value = request.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (given, text) = value.split_once(' ')?;
    given.eq_ignore_ascii_case(scheme).then(|| text.trim())
}

/// What the authenticator makes of a request, before its body is read: refused, or admitted
/// with the installation digest still to pin.
enum Admission {
    Refused,
    Admitted { pin: Option<[u8; 32]> },
}

fn admit(
    authenticator: &Authenticator,
    name: &str,
    request: &Request<Incoming>,
    certificate: Option<&[u8]>,
) -> Admission {
    // §mesh-22.5: the name the path claims is proved by the deployment's one authenticator, a
    // bearer token, an installation secret checked against its pin, or a client certificate.
    match authenticator {
        Authenticator::Bearer(check) => match credential(request, BEARER_SCHEME) {
            Some(token) if check(name, token) => Admission::Admitted { pin: None },
            _ => Admission::Refused,
        },
        Authenticator::Install(pins) => {
            let Some(secret) = credential(request, INSTALL_SCHEME).and_then(decode_install_secret)
            else {
                return Admission::Refused;
            };
            let digest = install_digest(&secret);
            match pins.pinned(name) {
                Some(held) if !same_digest(&held, &digest) => Admission::Refused,
                _ => Admission::Admitted { pin: Some(digest) },
            }
        }
        Authenticator::Mtls(check) => match certificate {
            Some(certificate) if check(name, certificate) => Admission::Admitted { pin: None },
            _ => Admission::Refused,
        },
    }
}

/// One exchange, in the order §mesh-22.3 gives: refused if a status of §mesh-22.4 says so, the
/// acknowledgement taken, the request's envelopes taken in, then answered with what the mailbox
/// holds.
async fn exchange(
    shared: Arc<Shared>,
    request: Request<Incoming>,
    certificate: Option<Arc<Vec<u8>>>,
) -> Response<Full<Bytes>> {
    // §mesh-22.4: the refusals below (`405`, `404`, `503`, `401`, `400`, `413`, `429`, `415`)
    // are each answered before the request's envelopes reach the core.
    if request.method() != Method::POST {
        let mut response = answer(StatusCode::METHOD_NOT_ALLOWED);
        response
            .headers_mut()
            .insert("Allow", HeaderValue::from_static("POST"));
        return response;
    }
    let settings = &shared.settings;
    let Some(name) = client_of(&settings.base_path, request.uri().path()) else {
        return answer(StatusCode::NOT_FOUND);
    };
    let name = name.to_string();
    if !(shared.is_bound)(&name) {
        return answer(StatusCode::NOT_FOUND);
    }
    if shared.exchanges.fetch_add(1, Ordering::AcqRel) >= settings.max_exchanges {
        shared.exchanges.fetch_sub(1, Ordering::AcqRel);
        return shed(StatusCode::SERVICE_UNAVAILABLE, settings.retry_after);
    }
    let _counted = Counted(&shared.exchanges);
    let pin = match admit(
        &shared.authenticator,
        &name,
        &request,
        certificate.as_deref().map(Vec::as_slice),
    ) {
        Admission::Refused => return unauthorized(&shared.authenticator),
        Admission::Admitted { pin } => pin,
    };
    let (Some(ack), Some(hold_ms), Some(acked_in)) = (
        number_header(&request, ACK_HEADER),
        number_header(&request, HOLD_HEADER),
        epoch_header(&request),
    ) else {
        return answer(StatusCode::BAD_REQUEST);
    };
    let declared = request
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if declared.is_some_and(|length| length > settings.max_body_bytes as u64) {
        return answer(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let typed = request
        .headers()
        .get(CONTENT_TYPE_NAME)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_string()
        });

    let in_flight = {
        let mut clients = shared.mailboxes.lock();
        let client = clients.entry(name.clone()).or_insert_with(Client::new);
        if client.in_flight >= settings.max_exchanges_per_client {
            return shed(StatusCode::TOO_MANY_REQUESTS, settings.retry_after);
        }
        client.in_flight += 1;
        client.last_seen = Instant::now();
        InFlight {
            mailboxes: Arc::clone(&shared.mailboxes),
            name: name.clone(),
        }
    };

    let body = match Limited::new(request.into_body(), settings.max_body_bytes)
        .collect()
        .await
    {
        Ok(collected) => collected.to_bytes(),
        Err(error) if error.downcast_ref::<LengthLimitError>().is_some() => {
            return answer(StatusCode::PAYLOAD_TOO_LARGE);
        }
        // The connection ended or broke before the body was whole; nobody reads the answer.
        Err(_) => return answer(StatusCode::BAD_REQUEST),
    };
    if !body.is_empty() && typed.as_deref() != Some(CONTENT_TYPE) {
        return answer(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    // §mesh-22.5: the name is pinned to the installation secret's SHA-256 in one step with the
    // check, so of two first exchanges that bring different secrets one is refused `401`.
    if let Some(digest) = pin {
        if let Authenticator::Install(pins) = &shared.authenticator {
            if !pins.pin(&name, digest) {
                return unauthorized(&shared.authenticator);
            }
        }
    }

    // Taken in: the acknowledgement, readiness and the envelopes, told in the order decided
    // (§mesh-22.3). The body is split as §mesh-22.2 frames it, a CBOR sequence whose unreadable
    // tail reaches the core as one last envelope.
    let (arrived, held_by) = {
        let telling = shared.telling.lock().await;
        let (became_ready, arrived) = {
            let mut clients = shared.mailboxes.lock();
            let client = clients.entry(name.clone()).or_insert_with(Client::new);
            client.acknowledge(ack, acked_in.as_deref(), &shared.mailboxes.epoch);
            let became_ready = !client.ready;
            client.ready = true;
            (became_ready, Arc::clone(&client.arrived))
        };
        let envelopes = split_sequence(&body);
        let carried = !envelopes.is_empty();
        let mut events = Vec::new();
        if became_ready {
            events.push(LinkEvent::Ready(name.clone()));
        }
        events.extend(
            envelopes
                .into_iter()
                .map(|envelope| LinkEvent::Received(name.clone(), envelope)),
        );
        let durability = if events.is_empty() {
            Durability::Now
        } else {
            tell(&shared, events).await
        };
        drop(telling);
        // Only what the request carried waits for a checkpoint: a poll took nothing in.
        (arrived, if carried { durability } else { Durability::Now })
    };

    // A checkpointing host's envelopes are acknowledged by a `200` only once a durable checkpoint
    // holds them (SCE_MESH.md section 10.11). Ones the host could not make durable, or that are
    // not durable within the longest hold, are answered as load shed: the client carries them
    // again, and the duplicate window drops what was taken.
    let durable = match held_by {
        Durability::Now => true,
        Durability::After(sealed) => durable_past(&shared, sealed, settings.max_hold).await,
        Durability::NotDurable => false,
    };
    if !durable {
        return shed(StatusCode::SERVICE_UNAVAILABLE, settings.retry_after);
    }

    // Held while the mailbox is empty, for at most the shorter of the two bounds: the long poll
    // of §mesh-22.3, answered as soon as an envelope arrives.
    let hold = Duration::from_millis(hold_ms).min(settings.max_hold);
    let until = tokio::time::Instant::now() + hold;
    loop {
        let notified = arrived.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let waiting = shared
            .mailboxes
            .lock()
            .get(&name)
            .is_some_and(|client| !client.entries.is_empty());
        if waiting || tokio::time::Instant::now() >= until {
            break;
        }
        tokio::select! {
            _ = &mut notified => {}
            _ = tokio::time::sleep_until(until) => {}
        }
    }

    let (first, body) = {
        let clients = shared.mailboxes.lock();
        match clients.get(&name) {
            Some(client) => (
                client.entries.front().map(|(number, _)| *number),
                super::join_sequence(client.entries.iter().map(|(_, bytes)| bytes.as_slice())),
            ),
            None => (None, Vec::new()),
        }
    };
    drop(in_flight);
    // §mesh-22.2: the body is every unacknowledged envelope as a CBOR sequence, `Sce-Mesh-First`
    // numbers its first (absent when it is empty), and every `200` names its incarnation.
    let mut response = Response::new(Full::new(Bytes::from(body)));
    response
        .headers_mut()
        .insert(CONTENT_TYPE_NAME, HeaderValue::from_static(CONTENT_TYPE));
    if let Ok(value) = HeaderValue::from_str(&shared.mailboxes.epoch) {
        response.headers_mut().insert(EPOCH_HEADER, value);
    }
    if let Some(first) = first {
        if let Ok(value) = HeaderValue::from_str(&first.to_string()) {
            response.headers_mut().insert(FIRST_HEADER, value);
        }
    }
    response
}

/// A TLS configuration for the server: `chain` (each certificate DER, the server's own first)
/// with `key` (DER: PKCS#8, PKCS#1 or SEC1), offering HTTP/2 and HTTP/1.1, on the cryptography
/// every binding runs on ([`crate::tls::provider`]). With `client_authorities` (each DER) every
/// client must present a certificate one of them issued, which is the `mtls` authenticator's
/// half that TLS keeps.
pub fn server_tls(
    chain: Vec<Vec<u8>>,
    key: Vec<u8>,
    client_authorities: Option<Vec<Vec<u8>>>,
) -> Result<Arc<rustls::ServerConfig>, crate::tls::Error> {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use rustls::{RootCertStore, ServerConfig};

    let key =
        PrivateKeyDer::try_from(key).map_err(|why| rustls::Error::General(why.to_string()))?;
    let chain: Vec<CertificateDer<'static>> = chain.into_iter().map(CertificateDer::from).collect();
    let builder = ServerConfig::builder_with_provider(crate::tls::provider())
        .with_safe_default_protocol_versions()?;
    let mut config = match client_authorities {
        None => builder.with_no_client_auth().with_single_cert(chain, key)?,
        Some(authorities) => {
            let mut roots = RootCertStore::empty();
            for authority in authorities {
                roots.add(CertificateDer::from(authority))?;
            }
            let verifier = WebPkiClientVerifier::builder_with_provider(
                Arc::new(roots),
                crate::tls::provider(),
            )
            .build()
            .map_err(|why| rustls::Error::General(why.to_string()))?;
            builder
                .with_client_cert_verifier(verifier)
                .with_single_cert(chain, key)?
        }
    };
    config.alpn_protocols = alloc::vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_is_named_by_the_last_segment_of_its_versioned_path() {
        assert_eq!(client_of("", "/sce-mesh/1/phone"), Some("phone"));
        assert_eq!(client_of("/api", "/api/sce-mesh/1/phone"), Some("phone"));
        assert_eq!(client_of("", "/sce-mesh/2/phone"), None, "another version");
        assert_eq!(client_of("", "/sce-mesh/1/a/b"), None, "not one segment");
        assert_eq!(client_of("", "/sce-mesh/1/"), None, "no name");
        assert_eq!(
            client_of("/api", "/sce-mesh/1/phone"),
            None,
            "not under the base"
        );
    }

    const EPOCH: &str = "00112233445566778899aabbccddeeff";
    const EARLIER: &str = "ffeeddccbbaa99887766554433221100";

    /// An acknowledgement drops what it names, and only when it counts in the incarnation the
    /// mailbox numbers in: one of another incarnation, or of none, names a numbering the mailbox
    /// does not hold.
    #[test]
    fn an_acknowledgement_drops_what_it_names_in_this_incarnation_and_no_more() {
        let mut client = Client::new();
        for n in 0..5u8 {
            client.entries.push_back((client.next, alloc::vec![n]));
            client.next += 1;
        }
        client.acknowledge(3, None, EPOCH);
        assert_eq!(
            client.entries.len(),
            5,
            "a client that has seen no incarnation"
        );
        client.acknowledge(3, Some(EARLIER), EPOCH);
        assert_eq!(client.entries.len(), 5, "an earlier incarnation's numbers");
        client.acknowledge(3, Some(EPOCH), EPOCH);
        assert_eq!(client.entries.front().map(|(n, _)| *n), Some(4));
        client.acknowledge(5, Some(EPOCH), EPOCH);
        assert!(client.entries.is_empty());
        assert_eq!(client.next, 6, "numbers go on where they were");
    }

    #[test]
    fn a_full_mailbox_refuses_as_a_full_queue_with_its_depth() {
        let mailboxes = Arc::new(Mailboxes {
            capacity: 2,
            epoch: EPOCH.to_string(),
            clients: Mutex::new(HashMap::new()),
        });
        let mut transport = MailboxTransport {
            mailboxes: Arc::clone(&mailboxes),
        };
        transport.transmit("phone", &[1]).unwrap();
        transport.transmit("phone", &[2]).unwrap();
        let refused = transport.transmit("phone", &[3]).unwrap_err();
        assert_eq!(refused.queue_full, Some(2));
        assert!(!refused.retryable);
        // Another client's mailbox is its own.
        transport.transmit("tablet", &[1]).unwrap();
        let clients = mailboxes.lock();
        let numbers: Vec<u64> = clients["phone"].entries.iter().map(|(n, _)| *n).collect();
        assert_eq!(numbers, [1, 2]);
    }

    fn accepted(
        name: &'static str,
        listen: &'static str,
        authenticator: HttpsAuthenticator,
        mailbox: u32,
    ) -> crate::peers::Peer {
        crate::peers::Peer {
            name,
            config: crate::router::PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: crate::inbound::Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: &[],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsAccept {
                listen,
                authenticator,
                max_hold_ms: 20_000,
                max_body_bytes: 4096,
                absence_ms: 60_000,
                mailbox,
            },
        }
    }

    /// A server built for one address serves every peer accepted there, under the smallest bound
    /// any of their bindings declares; peers on another address are another server's.
    #[test]
    fn an_address_is_served_under_the_smallest_bounds_its_bindings_declare() {
        let peers: &'static [crate::peers::Peer] = Box::leak(Box::new([
            accepted("phone", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 64),
            accepted("tablet", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 16),
            accepted("kiosk", "0.0.0.0:9443", HttpsAuthenticator::Mtls, 8),
        ]));
        let machine = Machine {
            name: "cloud",
            dedup_window: 8,
            gap_timeout_ms: 50,
            peers,
            sessions: None,
        };
        let listening = Listening::on(&machine, "0.0.0.0:8443").expect("two peers there");
        assert_eq!(listening.peers, ["phone", "tablet"]);
        assert_eq!(listening.authenticator, HttpsAuthenticator::Bearer);
        assert_eq!(listening.settings.mailbox, 16);
        assert_eq!(listening.settings.max_hold, Duration::from_millis(20_000));
        assert_eq!(listening.settings.max_body_bytes, 4096);
        assert_eq!(listening.settings.absence, Duration::from_millis(60_000));
        assert_eq!(
            Listening::on(&machine, "0.0.0.0:9443").map(|l| l.peers),
            Some(alloc::vec!["kiosk"])
        );
        assert!(Listening::on(&machine, "0.0.0.0:1").is_none());

        let mixed: &'static [crate::peers::Peer] = Box::leak(Box::new([
            accepted("phone", "0.0.0.0:8443", HttpsAuthenticator::Bearer, 64),
            accepted("tablet", "0.0.0.0:8443", HttpsAuthenticator::Install, 64),
        ]));
        let machine = Machine {
            peers: mixed,
            ..machine
        };
        assert!(
            Listening::on(&machine, "0.0.0.0:8443").is_none(),
            "two authenticators on one address"
        );
    }

    #[test]
    fn a_name_is_pinned_to_the_first_secret_and_refuses_any_other() {
        let pins = MemoryPins::default();
        let first = install_digest(&[1; 32]);
        let other = install_digest(&[2; 32]);
        assert_eq!(pins.pinned("phone"), None);
        assert!(pins.pin("phone", first));
        assert!(pins.pin("phone", first), "the same secret again");
        assert!(!pins.pin("phone", other));
        assert_eq!(pins.pinned("phone"), Some(first));
        assert!(pins.pin("tablet", other), "another name pins its own");
    }

    /// A host whose endpoint checkpoints, its endpoint sealed through `sealed` whenever envelopes
    /// are admitted; it counts the envelopes it is handed.
    struct Checkpointing {
        sealed: u64,
        received: Arc<AtomicUsize>,
    }

    impl ServerHost for Checkpointing {
        fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
            let carried = events
                .iter()
                .filter(|event| matches!(event, LinkEvent::Received(..)))
                .count();
            self.received.fetch_add(carried, Ordering::SeqCst);
            Durability::After(self.sealed)
        }
    }

    /// A host that could not make durable what it took in, as a keyed pool whose store failed
    /// the checkpoint's write reports it; it counts the envelopes it is handed.
    struct Unwritten {
        received: Arc<AtomicUsize>,
    }

    impl ServerHost for Unwritten {
        fn deliver(&self, events: Vec<LinkEvent>) -> Durability {
            let carried = events
                .iter()
                .filter(|event| matches!(event, LinkEvent::Received(..)))
                .count();
            self.received.fetch_add(carried, Ordering::SeqCst);
            Durability::NotDurable
        }
    }

    /// A server on a loopback port, the runtime it runs on, and the reference adapter that reaches
    /// it directly.
    struct Serving {
        // Held so that the server runs for as long as the test does.
        _runtime: tokio::runtime::Runtime,
        server: HttpsServer,
        address: std::net::SocketAddr,
        port: Arc<super::super::adapter::HyperTransport>,
    }

    /// A server of `phone` on a loopback port, with `host` behind it, holding an exchange at most
    /// `max_hold`.
    fn serving(host: impl ServerHost, max_hold: Duration) -> Serving {
        use super::super::adapter::{AdapterSettings, HyperTransport, ProxyPolicy};

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("a runtime");
        let server = HttpsServer::new(
            ServerSettings {
                max_hold,
                ..ServerSettings::default()
            },
            Authenticator::Bearer(Arc::new(|name: &str, token: &str| {
                name == "phone" && token == "t"
            })),
            |name| name == "phone",
            host,
        )
        .expect("a server");
        let listener = runtime
            .block_on(TcpListener::bind("127.0.0.1:0"))
            .expect("a loopback port");
        let address = listener.local_addr().expect("its address");
        runtime.spawn({
            let server = server.clone();
            async move {
                let _ = server.serve(listener, None).await;
            }
        });
        let port = Arc::new(
            HyperTransport::new(AdapterSettings {
                proxy: ProxyPolicy::Direct,
                ..AdapterSettings::default()
            })
            .expect("an adapter"),
        );
        Serving {
            _runtime: runtime,
            server,
            address,
            port,
        }
    }

    /// An exchange of `phone` with the server at `address`, carrying `body`, asking for no hold.
    fn request_to(address: std::net::SocketAddr, body: Vec<u8>) -> super::super::HttpsRequest {
        super::super::HttpsRequest {
            url: alloc::format!("http://{address}/sce-mesh/1/phone"),
            headers: alloc::vec![
                ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
                (ACK_HEADER.to_string(), "0".to_string()),
                (HOLD_HEADER.to_string(), "0".to_string()),
                ("Authorization".to_string(), "Bearer t".to_string()),
            ],
            body,
            timeout: Duration::from_secs(10),
        }
    }

    /// A server whose host's endpoint checkpoints answers an exchange that carried envelopes only
    /// once the host reports durable a checkpoint above the one sealed when they were admitted
    /// (SCE_MESH.md section 10.11); an exchange that carried none is answered at once, and every
    /// `200` names the server's incarnation.
    #[test]
    fn a_checkpointing_host_answers_once_a_checkpoint_holding_the_envelopes_is_durable() {
        use super::super::HttpsTransport;

        let received = Arc::new(AtomicUsize::new(0));
        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving(
            Checkpointing {
                sealed: 4,
                received: Arc::clone(&received),
            },
            Duration::from_secs(5),
        );
        assert!(is_epoch(server.epoch()), "{}", server.epoch());

        let poll = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(poll.status, 200, "a poll waits for no checkpoint");
        assert_eq!(poll.header(EPOCH_HEADER), Some(server.epoch()));

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn({
            let port = Arc::clone(&port);
            move || {
                let _ = tx.send(port.exchange(request_to(address, alloc::vec![0x01])));
            }
        });
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "answered before a checkpoint holding the envelope was durable"
        );
        server.durable(4);
        assert!(
            rx.recv_timeout(Duration::from_millis(300)).is_err(),
            "the checkpoint sealed before the envelope was admitted does not hold it"
        );
        server.durable(5);
        let answered = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("answered once durable")
            .expect("a response");
        assert_eq!(answered.status, 200);
        assert_eq!(answered.header(EPOCH_HEADER), Some(server.epoch()));
        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    /// An exchange whose envelopes the host could not make durable (its checkpoint's write failed,
    /// or a fence refused it) is answered `503` with `Retry-After` at once, as load shed, never
    /// `200` (SCE_MESH.md section 10.11): the client carries them again. A poll, which carries
    /// nothing, is answered `200` whatever the host says.
    #[test]
    fn an_exchange_whose_envelopes_the_host_could_not_make_durable_is_shed() {
        use super::super::HttpsTransport;

        let received = Arc::new(AtomicUsize::new(0));
        let max_hold = Duration::from_secs(5);
        let Serving {
            _runtime,
            server,
            address,
            port,
        } = serving(
            Unwritten {
                received: Arc::clone(&received),
            },
            max_hold,
        );

        let poll = port
            .exchange(request_to(address, Vec::new()))
            .expect("a response");
        assert_eq!(poll.status, 200, "a poll waits for no checkpoint");
        assert_eq!(poll.header(EPOCH_HEADER), Some(server.epoch()));

        for _ in 0..2 {
            let started = Instant::now();
            let shed = port
                .exchange(request_to(address, alloc::vec![0x01]))
                .expect("a response");
            assert_eq!(shed.status, 503, "acknowledged what is not durable");
            assert_eq!(shed.header("Retry-After"), Some("1"));
            assert_eq!(shed.header(EPOCH_HEADER), None, "a 503 numbers nothing");
            assert!(shed.body.is_empty());
            assert!(
                started.elapsed() < max_hold,
                "a host that could not make them durable is not waited for"
            );
        }
        assert_eq!(
            received.load(Ordering::SeqCst),
            2,
            "each copy reached the host"
        );
    }
}
