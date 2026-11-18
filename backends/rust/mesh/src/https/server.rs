// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The accepting side of the HTTPS binding (SCE_MESH.md section 22): the server machine's end,
//! where each client's envelopes wait in a mailbox until the client's next exchange carries them.
//!
//! hyper on tokio, HTTP/1.1 and HTTP/2 on one listener, TLS from [`server_tls`]. For each
//! exchange the server reads the client's name from the path, checks the authenticator, drops
//! what the client acknowledged, hands the core what the request carried and waits until the
//! macrosteps it caused have run ([`ServerHost::deliver`]), then answers with what the mailbox
//! holds, holding the exchange while it holds nothing (section 22.3).
//!
//! What the machine sends a client reaches the mailbox through [`MailboxTransport`], the
//! transport the host builds its endpoint on. A full mailbox refuses the envelope as a full queue
//! ([`TransportFailure::queue_full`]), which the core raises as SCE_MESH.md section 16.7 row 9.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::Infallible;
use core::sync::atomic::{AtomicUsize, Ordering};
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
    decode_install_secret, install_digest, is_path_segment, split_sequence, LinkEvent, ACK_HEADER,
    BEARER_SCHEME, CONTENT_TYPE, FIRST_HEADER, HOLD_HEADER, INSTALL_SCHEME, PATH_ROOT, VERSION,
};
use crate::endpoint::{Transport, TransportFailure};

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

/// A host's check of a bearer token: `(client name, token)`, true when the token proves the name.
pub type BearerCheck = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// A host's check of a client certificate: `(client name, the certificate's DER)`, true when the
/// certificate (which TLS already verified against the deployment's client authorities) names
/// the client.
pub type CertificateCheck = Arc<dyn Fn(&str, &[u8]) -> bool + Send + Sync>;

/// Who may claim a name (section 22.5). A binding with none is refused at deploy validation, so a
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

/// What the server asks of its host: tell the endpoint what the binding observed, run the
/// macrosteps it caused, and return once they have run. What the machine sent meanwhile is then
/// in the mailboxes, which is what lets an exchange answer with the replies its own request
/// caused.
///
/// Called on a thread of tokio's blocking pool, one call at a time, in the order the binding
/// observed the events.
pub trait ServerHost: Send + Sync + 'static {
    /// Hand `events` to the endpoint, in order ([`super::deliver`]), and run the machine.
    fn deliver(&self, events: Vec<LinkEvent>);
}

impl<F: Fn(Vec<LinkEvent>) + Send + Sync + 'static> ServerHost for F {
    fn deliver(&self, events: Vec<LinkEvent>) {
        self(events)
    }
}

const HELD: &str = "the server's state is never left mid-update: nothing in it panics while held";

/// One client's mailbox and presence.
struct Client {
    /// The envelopes not yet acknowledged, by number, oldest first; the numbers are consecutive.
    entries: VecDeque<(u64, Vec<u8>)>,
    /// The number the next envelope gets; the first is 1.
    next: u64,
    /// Whether the server has answered an exchange of this client since it started. The first
    /// exchange's acknowledgement names a mailbox of an earlier run, and is not read.
    answered: bool,
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
            answered: false,
            ready: false,
            in_flight: 0,
            last_seen: Instant::now(),
            arrived: Arc::new(Notify::new()),
        }
    }

    /// Drop what `ack` acknowledges (section 22.3). The first exchange's acknowledgement, and one
    /// above any number this mailbox gave, name a mailbox of an earlier run and drop nothing.
    fn acknowledge(&mut self, ack: u64) {
        if !self.answered || ack >= self.next {
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
    clients: Mutex<HashMap<String, Client>>,
}

impl Mailboxes {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Client>> {
        self.clients.lock().expect(HELD)
    }
}

/// The core's side of the server: what the machine sends a client waits in its mailbox.
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
}

/// The server: its mailboxes, its authenticator, and the host it tells.
#[derive(Clone)]
pub struct HttpsServer {
    shared: Arc<Shared>,
}

impl HttpsServer {
    /// A server with `settings`, letting a client in by `authenticator` when `is_bound` names it
    /// (the machine has a binding to it), and telling `host` what happens.
    pub fn new(
        settings: ServerSettings,
        authenticator: Authenticator,
        is_bound: impl Fn(&str) -> bool + Send + Sync + 'static,
        host: impl ServerHost,
    ) -> Self {
        let mailboxes = Arc::new(Mailboxes {
            capacity: settings.mailbox,
            clients: Mutex::new(HashMap::new()),
        });
        HttpsServer {
            shared: Arc::new(Shared {
                settings,
                authenticator,
                is_bound: Box::new(is_bound),
                host: Arc::new(host),
                mailboxes,
                telling: tokio::sync::Mutex::new(()),
                exchanges: AtomicUsize::new(0),
            }),
        }
    }

    /// The transport the host builds its endpoint on.
    pub fn transport(&self) -> MailboxTransport {
        MailboxTransport {
            mailboxes: Arc::clone(&self.shared.mailboxes),
        }
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
    /// dropped.
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

/// Hand `events` to the host on the blocking pool, and wait until it has run them.
async fn tell(shared: &Arc<Shared>, events: Vec<LinkEvent>) {
    let host = Arc::clone(&shared.host);
    // A host that panicked has left its endpoint in a state nobody can read; the exchange still
    // answers with what the mailbox holds, and the panic is the host's to see in its own log.
    let _ = tokio::task::spawn_blocking(move || host.deliver(events)).await;
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
/// version this server speaks; `None` is a `404`.
fn client_of<'a>(base_path: &str, path: &'a str) -> Option<&'a str> {
    let rest = path.strip_prefix(base_path)?.strip_prefix(PATH_ROOT)?;
    let (version, name) = rest.split_once('/')?;
    (version == VERSION && is_path_segment(name)).then_some(name)
}

/// A request header read as a number: absent is 0, anything but digits is `None` (a `400`).
fn number_header(request: &Request<Incoming>, name: &str) -> Option<u64> {
    match request.headers().get(name) {
        None => Some(0),
        Some(value) => value.to_str().ok()?.trim().parse().ok(),
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

/// One exchange (section 22.3).
async fn exchange(
    shared: Arc<Shared>,
    request: Request<Incoming>,
    certificate: Option<Arc<Vec<u8>>>,
) -> Response<Full<Bytes>> {
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
    let (Some(ack), Some(hold_ms)) = (
        number_header(&request, ACK_HEADER),
        number_header(&request, HOLD_HEADER),
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
    if let Some(digest) = pin {
        if let Authenticator::Install(pins) = &shared.authenticator {
            if !pins.pin(&name, digest) {
                return unauthorized(&shared.authenticator);
            }
        }
    }

    // Taken in: the acknowledgement, readiness and the envelopes, told in the order decided.
    let arrived = {
        let telling = shared.telling.lock().await;
        let (became_ready, arrived) = {
            let mut clients = shared.mailboxes.lock();
            let client = clients.entry(name.clone()).or_insert_with(Client::new);
            client.acknowledge(ack);
            client.answered = true;
            let became_ready = !client.ready;
            client.ready = true;
            (became_ready, Arc::clone(&client.arrived))
        };
        let mut events = Vec::new();
        if became_ready {
            events.push(LinkEvent::Ready(name.clone()));
        }
        events.extend(
            split_sequence(&body)
                .into_iter()
                .map(|envelope| LinkEvent::Received(name.clone(), envelope)),
        );
        if !events.is_empty() {
            tell(&shared, events).await;
        }
        drop(telling);
        arrived
    };

    // Held while the mailbox is empty, for at most the shorter of the two bounds.
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
    let mut response = Response::new(Full::new(Bytes::from(body)));
    response
        .headers_mut()
        .insert(CONTENT_TYPE_NAME, HeaderValue::from_static(CONTENT_TYPE));
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

    #[test]
    fn an_acknowledgement_drops_what_it_names_and_no_more() {
        let mut client = Client::new();
        for n in 0..5u8 {
            client.entries.push_back((client.next, alloc::vec![n]));
            client.next += 1;
        }
        // The first exchange of a run reads no acknowledgement: it names an earlier mailbox.
        client.acknowledge(3);
        assert_eq!(client.entries.len(), 5);
        client.answered = true;
        client.acknowledge(3);
        assert_eq!(client.entries.front().map(|(n, _)| *n), Some(4));
        // A number this mailbox never gave names an earlier run, and drops nothing.
        client.acknowledge(6);
        assert_eq!(client.entries.len(), 2);
        client.acknowledge(5);
        assert!(client.entries.is_empty());
        assert_eq!(client.next, 6, "numbers go on where they were");
    }

    #[test]
    fn a_full_mailbox_refuses_as_a_full_queue_with_its_depth() {
        let mailboxes = Arc::new(Mailboxes {
            capacity: 2,
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
}
