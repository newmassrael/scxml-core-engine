// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The dialing side of the HTTPS binding (SCE_MESH.md section 22): a machine that reaches one
//! server machine by exchanges it opens, through the port [`HttpsTransport`]. It is the client
//! of §mesh-22.8: [`HttpsClient`] over the port, and [`HttpsClientTransport`], the core's
//! transport.
//!
//! Every decision is of the module `client_core`, a state machine that moves nothing
//! (docs/adr/0117, D1). This module moves what it decides: it draws, signs, exchanges, waits,
//! reports and tells the host, as the core asks, and tells the core what came of it.
//!
//! Two threads, each with at most one exchange in flight, as §mesh-22.3 has a client keep them.
//! The poller holds an exchange open (`Sce-Mesh-Hold-Ms`) so that what the server sends reaches
//! the client as soon as it is made; the sender carries what the core transmits as soon as it is
//! transmitted, without waiting for a held exchange to end. Both acknowledge what the client has
//! received, and what either response carries is handed to the core once, in mailbox order.
//!
//! The binding keeps no decision of the core's. What it hands its host is [`LinkEvent`]s: the
//! server is ready (from the first exchange answered `200`), bytes arrived, an envelope the
//! exchange could not deliver (handed back as [`LinkEvent::Unsent`], which the core sends again
//! under the binding's retry policy or raises as SCE_MESH.md section 16.7 row 2 or row 3), the
//! link was interrupted (an exchange failed at the transport, while the server keeps the mailbox
//! and so every reply in it), the server started again without the mailbox (a response of a new
//! incarnation), and the server was lost (the client stopped, and takes nothing in any more).
//!
//! Message signatures (SCE_MESH.md section 22.9) are part of these semantics, so every adapter
//! gets them: under `verify:` ([`ClientSettings::verify`]) a `200` is taken only when a key of
//! the trust set signed it for the request it answers, and one that does not verify is taken
//! nothing of, acknowledges nothing, hands back what its request carried as not sent, and is told
//! once per edge ([`LinkEvent::Unauthenticated`], section 16.7 row 19); under the `key`
//! authenticator ([`Credential::Key`]) every request is signed by the client's key.

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sce_rust_crypto::cose::TrustedKey;
use sce_rust_crypto::random::SystemRandom;
use sce_rust_metrics::{Counter, Metrics};

pub use super::client_core::Received;
use super::client_core::{ClientCore, CoreSettings, Drawn, Effect, Input, Lane, Task, Wait};
use super::signature::{draw_nonce, normalize_target_uri, RequestSigner};
use super::{is_path_segment, until, Credential, HttpsTransport, LinkEvent, PATH_PREFIX};
use crate::endpoint::{Transport, TransportFailure};
use crate::peers::{is_instance_id, HttpsAuthenticator, Peer, PeerLink, Receipt};
use crate::reconnect::Backoff;

/// How a client reaches its server.
#[derive(Debug, Clone)]
pub struct ClientSettings {
    /// The server's base: `https://<host>[:<port>][<base path>]`, before the binding's path, with
    /// no query or fragment. `http://` is for a loopback host only.
    pub base: String,
    /// This machine's name: the path segment it posts under, and the `source` its envelopes carry.
    pub own: String,
    /// The server machine's name: the peer the core addresses.
    pub server: String,
    /// How the client proves its name (§mesh-22.5).
    pub credential: Credential,
    /// deploy.yaml's `hold_ms`: how long a poll asks the server to hold it while nothing waits.
    pub hold: Duration,
    /// How long an exchange may take beyond the hold it asks for before it is a timeout.
    pub exchange_timeout: Duration,
    /// The most bytes of envelopes one exchange carries; an envelope larger than this is carried
    /// alone. The server's `max_body_bytes` by default.
    pub max_body_bytes: usize,
    /// The waits between polls after a failed one.
    pub backoff: Backoff,
    /// deploy.yaml's `verify:`: the keys of the trust set it names, by id, when the binding
    /// verifies what its server sends (SCE_MESH.md section 22.9). `None` takes a `200` as it
    /// comes.
    pub verify: Option<Vec<TrustedKey>>,
    /// deploy.yaml's `receipt: durable` (docs/adr/0109, D10; SCE_MESH.md section 10.11): every
    /// request acknowledges only what the host reported a durable checkpoint of its own holds
    /// ([`HttpsClient::held`]), and each `200` that carried envelopes is told to the host
    /// ([`LinkEvent::Carried`]), whose core keeps each in its checkpoint until then.
    pub durable: bool,
    /// The instance this machine runs as, when it is an instance of an open machine
    /// (docs/adr/0113, D1 and D7): the host gives the id, and keeps it, and its core speaks as it
    /// (`Router::speak_as_instance`). Every exchange then posts to
    /// `<base>/sce-mesh/1/<own>/<instance>`; the client joins before it carries anything, makes
    /// every other exchange under the incarnation its server offered (`If-Match`) with its
    /// low-water mark (`Sce-Mesh-Mark`), and on a fence (`412`) tells the host what its server
    /// had not acknowledged ([`LinkEvent::Fenced`]) and joins anew. `None` for a machine that is
    /// not open.
    pub instance: Option<String>,
}

impl ClientSettings {
    /// The settings a deployment's defaults give a client of `own` reaching `server` at `base`.
    pub fn new(
        base: impl Into<String>,
        own: impl Into<String>,
        server: impl Into<String>,
        credential: Credential,
    ) -> Self {
        ClientSettings {
            base: base.into(),
            own: own.into(),
            server: server.into(),
            credential,
            hold: Duration::from_millis(u64::from(super::DEFAULT_HOLD_MS)),
            exchange_timeout: Duration::from_secs(30),
            max_body_bytes: super::DEFAULT_MAX_BODY_BYTES as usize,
            backoff: Backoff::default(),
            verify: None,
            durable: false,
            instance: None,
        }
    }

    /// The settings the generated peer table gives the machine `own` for `peer`, when this
    /// machine dials it over HTTPS ([`PeerLink::HttpsDial`]), proving its name with `credential`,
    /// which the host holds, and verifying what the server sends under `trusted`, the keys of the
    /// trust set the link's `verify` names, which the host holds too. `None` for any other link,
    /// for a credential of another kind than the link's `authenticator`, which the server would
    /// refuse, and for keys given to a link that verifies nothing or none to one that verifies:
    /// a deployment that asks for verification is never run without it.
    pub fn from_peer(
        own: &str,
        peer: &Peer,
        credential: Credential,
        trusted: Option<Vec<TrustedKey>>,
    ) -> Option<ClientSettings> {
        let PeerLink::HttpsDial {
            url,
            authenticator,
            hold_ms,
            verify,
            receipt,
        } = peer.link
        else {
            return None;
        };
        // §mesh-22.5: the credential is of the one authenticator the binding names, which is the
        // one the server checks.
        let fits = matches!(
            (authenticator, &credential),
            (HttpsAuthenticator::Bearer, Credential::Bearer(_))
                | (HttpsAuthenticator::Install, Credential::Install(_))
                | (HttpsAuthenticator::Mtls, Credential::Mtls)
                | (HttpsAuthenticator::Key, Credential::Key(_))
        );
        // SCE_MESH.md section 22.9: the trust set is given exactly when the link names one.
        let verifies = verify.is_some() == trusted.is_some();
        (fits && verifies).then(|| ClientSettings {
            hold: Duration::from_millis(u64::from(hold_ms)),
            verify: trusted,
            durable: matches!(receipt, Receipt::Durable { .. }),
            ..ClientSettings::new(url, own, peer.name, credential)
        })
    }
}

/// Why a client could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The machine name is not a path segment as it stands (§mesh-22.1).
    NotAPathSegment(String),
    /// The base is not an `https://` URL, or an `http://` one for a loopback host, or carries a
    /// query or a fragment.
    BadBase(String),
    /// The instance id is not 1 to 64 characters of `A-Z a-z 0-9 . _ -` (docs/adr/0113, D1).
    NotAnInstanceId(String),
}

/// What the two threads share.
struct Shared {
    /// The server machine's name: the peer the core addresses.
    server: String,
    /// The key every request is signed by, under the `key` authenticator.
    signer: Option<RequestSigner>,
    port: Arc<dyn HttpsTransport>,
    state: Mutex<State>,
    /// Wakes a thread when the core asks something of its lane.
    wake: Condvar,
    /// What the client reports each exchange to (docs/adr/0109, D16).
    metrics: Arc<dyn Metrics>,
}

type EventSink = Box<dyn FnMut(LinkEvent) + Send>;

struct State {
    core: ClientCore,
    /// What the core asked of each lane and its thread has not begun, oldest first.
    polls: VecDeque<Task>,
    sends: VecDeque<Task>,
    /// Where the host is told what happened. Called with the state held, so that what the two
    /// threads observe reaches the host in the order it was observed.
    events: EventSink,
}

const HELD: &str = "the client's state is never left mid-update: nothing in it panics while held";

impl State {
    fn tasks(&mut self, lane: Lane) -> &mut VecDeque<Task> {
        match lane {
            Lane::Poll => &mut self.polls,
            Lane::Send => &mut self.sends,
        }
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect(HELD)
    }

    /// Give the core `input`, with the state held, and do what it answers: report and tell the
    /// host here, in its order, and leave each lane's task to the lane's thread.
    fn step(&self, state: &mut State, input: Input) {
        for effect in state.core.step(input) {
            match effect {
                Effect::Count(status) => {
                    self.metrics.count(Counter::ClientExchanges { status }, 1);
                }
                Effect::Tell(event) => (state.events)(event),
                Effect::Ask(lane, task) => {
                    state.tasks(lane).push_back(task);
                    self.wake.notify_all();
                }
            }
        }
    }

    /// [`Self::step`], holding the state for it.
    fn give(&self, input: Input) {
        self.step(&mut self.lock(), input);
    }

    /// What the core asks of `lane` next, once it does.
    fn next_task(&self, lane: Lane) -> Task {
        let mut state = self.lock();
        loop {
            if let Some(task) = state.tasks(lane).pop_front() {
                return task;
            }
            state = self.wake.wait(state).expect(HELD);
        }
    }

    /// Wait `wait`, or less if the core asks something else of `lane` meanwhile. Returns whether
    /// it waited all of it.
    fn wait_out(&self, lane: Lane, wait: Duration) -> bool {
        let (mut state, _) = self
            .wake
            .wait_timeout_while(self.lock(), wait, |state| state.tasks(lane).is_empty())
            .expect(HELD);
        state.tasks(lane).is_empty()
    }
}

/// The core's side of the client: what it transmits waits for the sender's next exchange.
#[derive(Clone)]
pub struct HttpsClientTransport {
    shared: Arc<Shared>,
}

impl Transport for HttpsClientTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if peer != self.shared.server {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!("no HTTPS binding to {peer}")),
                queue_full: None,
            });
        }
        let mut state = self.shared.lock();
        if state.core.stopped() {
            return Err(TransportFailure {
                retryable: true,
                message: Some("the HTTPS client has stopped".to_string()),
                queue_full: None,
            });
        }
        self.shared
            .step(&mut state, Input::Transmitted(bytes.to_vec()));
        Ok(())
    }
}

/// A running client: its two threads, and the transport the core transmits through.
pub struct HttpsClient {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
}

impl HttpsClient {
    /// Start a client with `settings` over `port`, telling the host what happens through
    /// `events`, from the client's own threads. The first poll is made at once.
    pub fn start(
        settings: ClientSettings,
        port: Arc<dyn HttpsTransport>,
        events: impl FnMut(LinkEvent) + Send + 'static,
    ) -> Result<HttpsClient, ClientError> {
        Self::start_observed(settings, port, events, sce_rust_metrics::none())
    }

    /// [`Self::start`], reporting each exchange by the status its server answered to `metrics`
    /// (docs/adr/0109, D16).
    pub fn start_observed(
        settings: ClientSettings,
        port: Arc<dyn HttpsTransport>,
        events: impl FnMut(LinkEvent) + Send + 'static,
        metrics: Arc<dyn Metrics>,
    ) -> Result<HttpsClient, ClientError> {
        if !is_path_segment(&settings.own) {
            return Err(ClientError::NotAPathSegment(settings.own));
        }
        let base = settings.base.trim_end_matches('/');
        if !base_is_allowed(base) {
            return Err(ClientError::BadBase(settings.base));
        }
        // §mesh-22.1: every exchange posts to `<base>/sce-mesh/1/<client>`; the path is the only
        // place the client's machine name travels. An instance of an open machine posts under the
        // pair `<machine>/<instance>` (docs/adr/0113, D2).
        let url = match &settings.instance {
            None => alloc::format!("{base}{PATH_PREFIX}{}", settings.own),
            Some(instance) if is_instance_id(instance) => {
                alloc::format!("{base}{PATH_PREFIX}{}/{instance}", settings.own)
            }
            Some(instance) => return Err(ClientError::NotAnInstanceId(instance.clone())),
        };
        let Some(target_uri) = normalize_target_uri(&url) else {
            return Err(ClientError::BadBase(settings.base));
        };
        let signer = settings.credential.request_signer().cloned();
        let core = ClientCore::new(CoreSettings {
            server: settings.server.clone(),
            url,
            target_uri,
            authorization: settings.credential.authorization(),
            signs: signer.is_some(),
            verify: settings.verify,
            hold: settings.hold,
            exchange_timeout: settings.exchange_timeout,
            max_body_bytes: settings.max_body_bytes,
            backoff: settings.backoff,
            durable: settings.durable,
            instance: settings.instance.is_some(),
        });
        let shared = Arc::new(Shared {
            server: settings.server,
            signer,
            port,
            state: Mutex::new(State {
                core,
                polls: VecDeque::new(),
                sends: VecDeque::new(),
                events: Box::new(events),
            }),
            wake: Condvar::new(),
            metrics,
        });
        shared.give(Input::Started);
        let poller = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || run(&shared, Lane::Poll))
        };
        let sender = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || run(&shared, Lane::Send))
        };
        Ok(HttpsClient {
            shared,
            threads: alloc::vec![poller, sender],
        })
    }

    /// The transport the core transmits to the server through.
    pub fn transport(&self) -> HttpsClientTransport {
        HttpsClientTransport {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Where the client stands in its server's mailbox: the incarnation and the highest number
    /// it handed its host (docs/adr/0109, D10). A host that checkpoints under `receipt: durable`
    /// reads this before it applies what the client handed it, so that what it applies holds at
    /// least this much, and reports it with [`Self::held`] once a checkpoint taken after is
    /// durable.
    pub fn received(&self) -> Received {
        self.shared.lock().core.received()
    }

    /// A durable checkpoint of the host holds every envelope up to `received`, read with
    /// [`Self::received`] before the host applied what it was handed (docs/adr/0109, D10): under
    /// `receipt: durable` every request acknowledges that much from now on, so the server
    /// deletes nothing a crash of this machine would lose. A mark of an incarnation the client
    /// has left counts in a numbering that is gone, and moves nothing; on a binding that is not
    /// durable this does nothing.
    pub fn held(&self, received: &Received) {
        self.shared.give(Input::Held(received.clone()));
    }

    /// Stop the client: no exchange is begun after this, what waits in the outbox is handed back
    /// as [`LinkEvent::Unsent`], and the server is lost, ready or interrupted, since nothing it
    /// keeps for this machine is taken in any more. Returns once both threads ended, which is once
    /// the exchange each has in flight ended.
    pub fn stop(mut self) {
        self.signal_stop();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    fn signal_stop(&self) {
        self.shared.give(Input::Stop);
    }
}

impl Drop for HttpsClient {
    /// A client dropped without [`stop`](Self::stop) stops as `stop` does, without waiting for its
    /// threads: each ends after the exchange it has in flight.
    fn drop(&mut self) {
        self.signal_stop();
    }
}

/// Whether `base` is a URL the binding posts to: `https://` with a host, or `http://` with a
/// loopback host, and in either no query or fragment (§mesh-22.1: `http://` without TLS is for a
/// loopback test only).
pub(super) fn base_is_allowed(base: &str) -> bool {
    if base.contains(['?', '#']) {
        return false;
    }
    if let Some(rest) = base.strip_prefix("https://") {
        return !authority(rest).is_empty();
    }
    match base.strip_prefix("http://") {
        Some(rest) => is_loopback(host_of(authority(rest))),
        None => false,
    }
}

/// The authority of what follows a URL's scheme: everything before the first `/`.
fn authority(rest: &str) -> &str {
    rest.split('/').next().unwrap_or_default()
}

/// The host of an authority, without its port and without the brackets of an IPv6 literal.
fn host_of(authority: &str) -> &str {
    let without_user = authority.rsplit('@').next().unwrap_or_default();
    if let Some(literal) = without_user.strip_prefix('[') {
        return literal.split(']').next().unwrap_or_default();
    }
    without_user.split(':').next().unwrap_or_default()
}

fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// A lane's thread: do what the core asks of the lane, one task after another, and tell it what
/// came of each, until it asks the lane to stop.
fn run(shared: &Shared, lane: Lane) {
    loop {
        let input = match shared.next_task(lane) {
            Task::Draw => Input::Drawn(
                lane,
                draw_nonce(&mut SystemRandom).map(|nonce| Drawn {
                    nonce,
                    moment: since_epoch(),
                }),
            ),
            // §mesh-22.9: the key of the `key` authenticator is held here, and signs at once.
            Task::Sign {
                target_uri,
                headers,
                created,
            } => Input::Signed(
                lane,
                shared
                    .signer
                    .as_ref()
                    .and_then(|signer| signer.sign(&target_uri, &headers, created))
                    .map(Vec::from),
            ),
            Task::Carry(request) => Input::Answered(lane, shared.port.exchange(request)),
            Task::Wake(wait) => {
                if !shared.wait_out(lane, length_of(wait)) {
                    continue;
                }
                Input::Woken(lane)
            }
            Task::Stop => return,
        };
        shared.give(input);
    }
}

/// The moment, since the Unix epoch; none for a clock set before it.
fn since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}

/// How long `wait` is from now.
fn length_of(wait: Wait) -> Duration {
    match wait {
        Wait::For(delay) => delay,
        // A backoff of this one wait, for its spread.
        Wait::Spread(delay) => Backoff {
            initial: delay,
            max: delay,
            factor: 1,
            jitter: true,
        }
        .delay(0),
        Wait::Until(moment) => until(moment, SystemTime::now()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::signature::{
        content_digest, field_value, Answered, CONTENT_DIGEST_HEADER, NONCE_HEADER,
    };
    use super::super::{
        HttpsFailure, HttpsRequest, HttpsResponse, ACK_HEADER, CONTENT_TYPE, EPOCH_HEADER,
        FIRST_HEADER, HOLD_HEADER,
    };
    use super::*;
    use sce_rust_metrics::ExchangeOutcome;
    use std::sync::mpsc;

    type Answers = Mutex<VecDeque<Result<HttpsResponse, HttpsFailure>>>;

    /// A port that answers from two scripts, one for the polls (requests that carry nothing) and
    /// one for the requests that carry envelopes, so a test can aim an answer at either thread;
    /// and that records what it was asked.
    #[derive(Default)]
    struct Scripted {
        polls: Answers,
        sends: Answers,
        asked: Mutex<Vec<HttpsRequest>>,
    }

    impl HttpsTransport for Scripted {
        fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
            self.asked.lock().unwrap().push(request.clone());
            let is_poll = request.body.is_empty();
            let script = if is_poll { &self.polls } else { &self.sends };
            let next = script.lock().unwrap().pop_front();
            match next {
                Some(answer) => answer,
                // Out of script: an exchange taken in with nothing to answer, after a little
                // while for a poll, so a test's loop does not spin.
                None => {
                    if is_poll {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Ok(ok(None, Vec::new()))
                }
            }
        }
    }

    /// The incarnation the scripted server's mailbox numbers in, and one it numbered in before it
    /// started again.
    const EPOCH: &str = "0123456789abcdef0123456789abcdef";
    const EARLIER: &str = "fedcba9876543210fedcba9876543210";

    /// A `200` of the mailbox incarnation `epoch`, its body's first number `first`.
    fn ok_in(epoch: &str, first: Option<u64>, body: Vec<u8>) -> HttpsResponse {
        let mut headers = alloc::vec![(EPOCH_HEADER.to_string(), epoch.to_string())];
        if let Some(first) = first {
            headers.push((FIRST_HEADER.to_string(), first.to_string()));
        }
        HttpsResponse {
            status: 200,
            headers,
            body,
        }
    }

    fn ok(first: Option<u64>, body: Vec<u8>) -> HttpsResponse {
        ok_in(EPOCH, first, body)
    }

    /// The value of the header `name` of `request`.
    fn header_of(request: &HttpsRequest, name: &str) -> Option<String> {
        request
            .headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
    }

    fn status(code: u16, headers: &[(&str, &str)]) -> HttpsResponse {
        HttpsResponse {
            status: code,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: Vec::new(),
        }
    }

    fn settings() -> ClientSettings {
        ClientSettings {
            hold: Duration::from_millis(500),
            backoff: Backoff {
                initial: Duration::from_millis(1),
                max: Duration::from_millis(1),
                factor: 1,
                jitter: false,
            },
            ..ClientSettings::new(
                "https://cal.example/api",
                "phone",
                "cloud",
                Credential::Bearer("t".to_string()),
            )
        }
    }

    /// Three one-byte CBOR items (the unsigned integers 1, 2 and 3), which split as three.
    fn three() -> Vec<u8> {
        alloc::vec![0x01, 0x02, 0x03]
    }

    /// How long each wait the core asks is: as it says, spread by up to 20 percent either way, and
    /// until a moment, none when it is past. Red if a spread wait is waited as it stands, or a
    /// date is read as a delay.
    #[test]
    fn a_wait_is_as_long_as_the_core_asks() {
        let delay = Duration::from_secs(10);
        assert_eq!(length_of(Wait::For(delay)), delay);
        let spread: Vec<Duration> = (0..64).map(|_| length_of(Wait::Spread(delay))).collect();
        assert!(spread
            .iter()
            .all(|wait| (Duration::from_secs(8)..=Duration::from_secs(12)).contains(wait)));
        assert!(spread.iter().any(|wait| *wait != delay), "{spread:?}");
        assert_eq!(
            length_of(Wait::Until(Duration::from_secs(1))),
            Duration::ZERO
        );
        let ahead = length_of(Wait::Until(since_epoch() + delay));
        assert!(ahead > Duration::ZERO && ahead <= delay, "{ahead:?}");
    }

    fn dialed(authenticator: HttpsAuthenticator) -> Peer {
        Peer {
            name: "cloud",
            config: crate::router::PeerConfig {
                transport: "https",
                buffer: None,
                retry: None,
                stamp_sequence: false,
                delivery: crate::inbound::Delivery {
                    dedup: true,
                    ordered: false,
                },
                responders: crate::router::Responders::Named(&["cloud"]),
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsDial {
                url: "https://cal.example/api",
                authenticator,
                hold_ms: 9_000,
                verify: None,
                receipt: Receipt::Transport,
            },
        }
    }

    /// A peer table's dialing link gives the client its URL, its server's name and its hold, and
    /// only with a credential of the kind the link's authenticator names.
    #[test]
    fn a_dialing_link_gives_the_settings_and_takes_only_its_own_credential() {
        let peer = dialed(HttpsAuthenticator::Install);
        let settings =
            ClientSettings::from_peer("phone", &peer, Credential::Install([1; 32]), None)
                .expect("an installation secret for an `install` link");
        assert_eq!(settings.base, "https://cal.example/api");
        assert_eq!(settings.own, "phone");
        assert_eq!(settings.server, "cloud");
        assert_eq!(settings.hold, Duration::from_millis(9_000));
        assert_eq!(settings.verify, None);
        assert!(ClientSettings::from_peer("phone", &peer, Credential::Mtls, None).is_none());
        let bearer = dialed(HttpsAuthenticator::Bearer);
        assert!(
            ClientSettings::from_peer("phone", &bearer, Credential::Bearer("t".into()), None)
                .is_some()
        );
        let accepted = Peer {
            link: PeerLink::ZenohInbox,
            ..bearer
        };
        assert!(ClientSettings::from_peer(
            "phone",
            &accepted,
            Credential::Bearer("t".into()),
            None
        )
        .is_none());
    }

    /// docs/adr/0109, D9: a link that names `verify` gives the client the trust set's keys and is
    /// refused without them, one that names none is refused keys, and the `key` authenticator
    /// takes a key credential. Red if a deployment that asks for verification can be run without
    /// it.
    #[test]
    fn a_dialing_link_that_verifies_is_never_run_without_its_trust_set() {
        let keys = alloc::vec![TrustedKey {
            id: "server-2026".to_string(),
            public_key: [7u8; 32],
        }];
        let verifying = Peer {
            link: PeerLink::HttpsDial {
                url: "https://cal.example/api",
                authenticator: HttpsAuthenticator::Key,
                hold_ms: 9_000,
                verify: Some("servers"),
                receipt: Receipt::Transport,
            },
            ..dialed(HttpsAuthenticator::Key)
        };
        let key = || {
            Credential::Key(super::super::signature::RequestSigner::new(
                sce_rust_crypto::ed25519::KeyPair::from_seed(&[5u8; 32]),
            ))
        };
        let settings = ClientSettings::from_peer("phone", &verifying, key(), Some(keys.clone()))
            .expect("a key and the trust set");
        assert_eq!(settings.verify, Some(keys.clone()));
        assert!(
            ClientSettings::from_peer("phone", &verifying, key(), None).is_none(),
            "a link that verifies, given no trust set"
        );
        assert!(
            ClientSettings::from_peer("phone", &dialed(HttpsAuthenticator::Key), key(), Some(keys))
                .is_none(),
            "keys for a link that verifies nothing"
        );
        assert!(
            ClientSettings::from_peer(
                "phone",
                &dialed(HttpsAuthenticator::Key),
                Credential::Bearer("t".into()),
                None
            )
            .is_none(),
            "a token for the `key` authenticator"
        );
    }

    #[test]
    fn a_base_is_https_or_loopback_http_without_query_or_fragment() {
        for good in [
            "https://cal.example",
            "https://cal.example:8443/api",
            "http://127.0.0.1:9000",
            "http://localhost/base",
            "http://[::1]:80",
        ] {
            assert!(base_is_allowed(good), "{good}");
        }
        for bad in [
            "https://",
            "http://cal.example",
            "http://10.0.0.1:80",
            "https://cal.example/?q",
            "https://cal.example/#f",
            "wss://cal.example",
            "cal.example",
        ] {
            assert!(!base_is_allowed(bad), "{bad}");
        }
    }

    #[test]
    fn a_client_is_refused_a_name_or_a_base_the_binding_cannot_post_under() {
        let port: Arc<dyn HttpsTransport> = Arc::new(Scripted::default());
        let bad_name = ClientSettings {
            own: "a/b".to_string(),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_name, Arc::clone(&port), |_| {}),
            Err(ClientError::NotAPathSegment(_))
        ));
        let bad_instance = ClientSettings {
            instance: Some("a b".to_string()),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_instance, Arc::clone(&port), |_| {}),
            Err(ClientError::NotAnInstanceId(_))
        ));
        let bad_base = ClientSettings {
            base: "http://cal.example".to_string(),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_base, port, |_| {}),
            Err(ClientError::BadBase(_))
        ));
    }

    /// docs/adr/0113, D2 and D4: an instance of an open machine posts under its pair, joins first
    /// with a nonce and no incarnation, and makes its next exchange under the incarnation the join
    /// was offered. Red if `start` posts under the machine's name alone, or the shell passes the
    /// core no `instance`.
    #[test]
    fn an_instance_posts_under_its_pair_and_joins_before_anything_else() {
        let port = Arc::new(Scripted::default());
        port.polls.lock().unwrap().push_back(Ok(status(
            200,
            &[(
                super::super::ETAG_HEADER,
                super::super::entity_tag(7).as_str(),
            )],
        )));
        let client = HttpsClient::start(
            ClientSettings {
                instance: Some("a1".to_string()),
                ..settings()
            },
            Arc::clone(&port) as Arc<dyn HttpsTransport>,
            |_| {},
        )
        .expect("a client of an instance");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while port.asked.lock().unwrap().len() < 2 {
            assert!(std::time::Instant::now() < deadline, "a second exchange");
            std::thread::sleep(Duration::from_millis(5));
        }
        client.stop();
        let asked = port.asked.lock().unwrap();
        assert_eq!(asked[0].url, "https://cal.example/api/sce-mesh/1/phone/a1");
        assert!(header_of(&asked[0], super::super::signature::NONCE_HEADER).is_some());
        assert_eq!(header_of(&asked[0], super::super::IF_MATCH_HEADER), None);
        assert_eq!(
            header_of(&asked[1], super::super::IF_MATCH_HEADER),
            Some(super::super::entity_tag(7))
        );
        assert_eq!(
            header_of(&asked[1], super::super::MARK_HEADER).as_deref(),
            Some("1")
        );
    }

    fn next(rx: &mpsc::Receiver<LinkEvent>) -> LinkEvent {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("a link event within five seconds")
    }

    /// The first exchange answered `200` makes the server ready, a transport failure interrupts
    /// it (section 22.4: the mailbox and every reply in it stay on the server, so it is not lost)
    /// and hands back what the failed exchange carried, and the next `200` makes it ready again.
    #[test]
    fn readiness_follows_the_exchanges_and_a_failure_hands_back_what_it_carried() {
        let port = Arc::new(Scripted::default());
        port.polls.lock().unwrap().extend([
            Ok(ok(None, Vec::new())),
            Err(HttpsFailure::Connect),
            Ok(ok(Some(1), alloc::vec![0x05])),
        ]);
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert!(matches!(next(&rx), LinkEvent::Interrupted(peer, _) if peer == "cloud"));
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        let first = &asked[0];
        assert_eq!(first.url, "https://cal.example/api/sce-mesh/1/phone");
        assert!(first.body.is_empty(), "a poll carries nothing");
        let header = |name: &str| {
            first
                .headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        };
        assert_eq!(header("Content-Type").as_deref(), Some(CONTENT_TYPE));
        assert_eq!(header(ACK_HEADER).as_deref(), Some("0"));
        assert_eq!(header(HOLD_HEADER).as_deref(), Some("500"));
        assert_eq!(header("Authorization").as_deref(), Some("Bearer t"));
        assert_eq!(
            first.timeout,
            Duration::from_millis(500) + Duration::from_secs(30)
        );
        // The exchange after the one that took mailbox number 1 acknowledges it.
        assert!(asked[3..].iter().all(|request| request
            .headers
            .contains(&(ACK_HEADER.to_string(), "1".to_string()))));
    }

    /// A client that stops makes no exchange any more, so nothing the server keeps for it is
    /// taken in: its server is lost though the link was only interrupted when it stopped, and a
    /// request the core still waits on is let go. Red if a stop loses only a ready server.
    #[test]
    fn a_client_that_stops_while_interrupted_loses_its_server() {
        let port = Arc::new(Scripted::default());
        port.polls
            .lock()
            .unwrap()
            .extend([Ok(ok(None, Vec::new())), Err(HttpsFailure::Connect)]);
        let (tx, rx) = mpsc::channel();
        // A long backoff after the failure, so no poll makes the server ready again before the
        // stop; the stop cuts the wait short.
        let slow = ClientSettings {
            backoff: Backoff {
                initial: Duration::from_secs(60),
                max: Duration::from_secs(60),
                factor: 1,
                jitter: false,
            },
            ..settings()
        };
        let client = HttpsClient::start(slow, port, move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert!(matches!(next(&rx), LinkEvent::Interrupted(peer, _) if peer == "cloud"));
        client.stop();
        assert!(matches!(next(&rx), LinkEvent::Lost(peer, _) if peer == "cloud"));
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "lost once"
        );
    }

    /// A server that started again names a new incarnation: the client's acknowledgement starts
    /// over in it, the first envelope of the new numbering reaches the core though its number is
    /// one the client acknowledged before, a response that carries it again does not hand it on a
    /// second time, and every request names the incarnation its acknowledgement counts in.
    #[test]
    fn a_new_incarnation_starts_the_acknowledgement_over_and_delivers_once() {
        let port = Arc::new(Scripted::default());
        port.polls.lock().unwrap().extend([
            Ok(ok_in(EARLIER, Some(1), alloc::vec![0x05])),
            Ok(ok(Some(1), alloc::vec![0x06])),
            Ok(ok(Some(1), alloc::vec![0x06])),
        ]);
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        assert_eq!(next(&rx), LinkEvent::Restarted("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x06])
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while port.asked.lock().unwrap().len() < 4 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "carried again, and handed on once"
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        assert!(asked.len() >= 4, "{} exchanges", asked.len());
        let acknowledged = |at: usize| {
            (
                header_of(&asked[at], ACK_HEADER),
                header_of(&asked[at], EPOCH_HEADER),
            )
        };
        assert_eq!(acknowledged(0), (Some("0".to_string()), None));
        assert_eq!(
            acknowledged(1),
            (Some("1".to_string()), Some(EARLIER.to_string()))
        );
        assert_eq!(
            acknowledged(2),
            (Some("1".to_string()), Some(EPOCH.to_string()))
        );
        assert_eq!(
            acknowledged(3),
            (Some("1".to_string()), Some(EPOCH.to_string()))
        );
    }

    /// What the core transmits is carried by an exchange that asks for no hold; a refusal the
    /// server answered hands it back unsent and leaves the server ready.
    #[test]
    fn a_transmitted_envelope_is_carried_at_once_and_a_refusal_hands_it_back() {
        let port = Arc::new(Scripted::default());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));

        port.sends
            .lock()
            .unwrap()
            .push_back(Ok(status(401, &[("WWW-Authenticate", "Bearer")])));
        let mut transport = client.transport();
        transport.transmit("cloud", &[0x09]).unwrap();
        assert_eq!(
            next(&rx),
            LinkEvent::Unsent("cloud".to_string(), alloc::vec![0x09])
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "a refusal is not a lost server"
        );
        let carried = port
            .asked
            .lock()
            .unwrap()
            .iter()
            .find(|request| !request.body.is_empty())
            .cloned()
            .expect("an exchange carried the envelope");
        assert_eq!(carried.body, [0x09]);
        assert!(carried
            .headers
            .contains(&(HOLD_HEADER.to_string(), "0".to_string())));

        assert!(
            transport.transmit("elsewhere", &[0x01]).is_err(),
            "the binding reaches its one server"
        );
        client.stop();
        assert!(matches!(next(&rx), LinkEvent::Lost(peer, _) if peer == "cloud"));
        assert!(transport.transmit("cloud", &[0x01]).is_err(), "stopped");
    }

    /// docs/adr/0109, D10: under `receipt: durable` a client acknowledges only what its host
    /// reported a durable checkpoint holds, never merely what it handed on, and a mark of an
    /// incarnation it left moves nothing; each `200` that carried what the core transmitted is
    /// told to the host as `Carried`. Red if `ClientCore::acknowledging` answers `mark` alone, or the
    /// `200` branch of `take_outcome` tells no `Carried`.
    #[test]
    fn a_durable_client_acknowledges_only_what_its_host_holds_and_tells_what_a_200_carried() {
        let port = Arc::new(Scripted::default());
        port.polls
            .lock()
            .unwrap()
            .push_back(Ok(ok(Some(1), three())));
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(
            ClientSettings {
                durable: true,
                ..settings()
            },
            port.clone(),
            move |event| {
                let _ = tx.send(event);
            },
        )
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        for _ in 0..3 {
            assert!(matches!(next(&rx), LinkEvent::Received(..)));
        }
        let acknowledged = |port: &Scripted| {
            std::thread::sleep(Duration::from_millis(150));
            port.asked
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|request| request.body.is_empty())
                .and_then(|request| header_of(request, ACK_HEADER))
        };
        assert_eq!(
            acknowledged(&port),
            Some("0".to_string()),
            "nothing a checkpoint holds yet"
        );
        let received = client.received();
        assert_eq!(
            received,
            Received {
                epoch: Some(EPOCH.to_string()),
                number: 3
            }
        );
        client.held(&Received {
            epoch: Some(EARLIER.to_string()),
            number: 3,
        });
        assert_eq!(
            acknowledged(&port),
            Some("0".to_string()),
            "a mark of an incarnation the client left moves nothing"
        );
        client.held(&received);
        assert_eq!(acknowledged(&port), Some("3".to_string()));

        let mut transport = client.transport();
        transport.transmit("cloud", &[0x09]).unwrap();
        assert_eq!(
            next(&rx),
            LinkEvent::Carried("cloud".to_string(), alloc::vec![alloc::vec![0x09]])
        );
        client.stop();
    }

    /// A server that sheds load is waited out, and what the shed exchange carried goes again; it
    /// is neither unsent nor a lost server.
    #[test]
    fn a_shed_exchange_goes_again_after_retry_after() {
        let port = Arc::new(Scripted::default());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(settings(), port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        {
            let mut sends = port.sends.lock().unwrap();
            sends.push_back(Ok(status(503, &[("Retry-After", "0")])));
            sends.push_back(Ok(ok(None, Vec::new())));
        }
        client.transport().transmit("cloud", &[0x0a]).unwrap();
        let carried = || {
            port.asked
                .lock()
                .unwrap()
                .iter()
                .filter(|request| request.body == [0x0a])
                .count()
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while carried() < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(carried(), 2, "carried, shed, carried again");
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "nothing to tell the host"
        );
        client.stop();
    }

    use super::super::signature::{
        digest_matches, RequestSignature, RequestSigner, ResponseSigner,
    };
    use sce_rust_crypto::ed25519::KeyPair;

    /// A port that answers every exchange as a server under `sign:` does: mailbox number 1,
    /// `[0x05]`, in [`EPOCH`] until a request acknowledges it, then nothing, each response signed
    /// for the request it answers. The first `tampered` responses that carry the envelope have
    /// `Sce-Mesh-First` rewritten after signing, as an intermediary that alters the exchange
    /// would. Records what it was asked.
    struct SigningPort {
        signer: ResponseSigner,
        tampered: Mutex<usize>,
        asked: Mutex<Vec<HttpsRequest>>,
    }

    impl HttpsTransport for SigningPort {
        fn exchange(&self, request: HttpsRequest) -> Result<HttpsResponse, HttpsFailure> {
            self.asked.lock().unwrap().push(request.clone());
            let header = |name: &str| field_value(&request.headers, name).unwrap_or_default();
            let acknowledged = header(ACK_HEADER).parse::<u64>().unwrap_or(0) >= 1;
            let body = if acknowledged {
                // An empty mailbox holds a poll a little, so a test's loop does not spin.
                std::thread::sleep(Duration::from_millis(20));
                Vec::new()
            } else {
                alloc::vec![0x05]
            };
            let first = (!body.is_empty()).then_some("1");
            let answered = Answered {
                target_uri: normalize_target_uri(&request.url).expect("a URL the client posts to"),
                ack: header(ACK_HEADER),
                nonce: header(NONCE_HEADER),
                if_match: None,
            };
            let digest = content_digest(&body);
            let mut headers = alloc::vec![
                (EPOCH_HEADER.to_string(), EPOCH.to_string()),
                (CONTENT_DIGEST_HEADER.to_string(), digest.clone()),
            ];
            if let Some(first) = first {
                headers.push((FIRST_HEADER.to_string(), first.to_string()));
            }
            headers.extend(self.signer.sign(&digest, EPOCH, first, None, &answered));
            let mut tampered = self.tampered.lock().unwrap();
            if first.is_some() && *tampered > 0 {
                *tampered -= 1;
                for (name, value) in headers.iter_mut() {
                    if name == FIRST_HEADER {
                        *value = "2".to_string();
                    }
                }
            }
            Ok(HttpsResponse {
                status: 200,
                headers,
                body,
            })
        }
    }

    /// SCE_MESH.md section 22.9: under `verify:`, two responses whose `Sce-Mesh-First` an
    /// intermediary rewrote are taken nothing of: no envelope reaches the core, the server is not
    /// made ready, every request meanwhile acknowledges nothing, and the host is told once (row
    /// 19). The next response, which verifies, is taken as it is. Red if the client takes an
    /// unverified `200` (the envelope would arrive numbered 2 and be acknowledged), or tells each
    /// one.
    #[test]
    fn a_response_that_does_not_verify_is_taken_nothing_of_and_told_once() {
        let key = KeyPair::from_seed(&[3u8; 32]);
        let port = Arc::new(SigningPort {
            signer: ResponseSigner::new("server-2026", key.clone()),
            tampered: Mutex::new(2),
            asked: Mutex::new(Vec::new()),
        });
        let verifying = ClientSettings {
            verify: Some(alloc::vec![TrustedKey {
                id: "server-2026".to_string(),
                public_key: key.public_key(),
            }]),
            ..settings()
        };
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(verifying, port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert!(
            matches!(next(&rx), LinkEvent::Unauthenticated(peer, _) if peer == "cloud"),
            "told once, before anything is taken"
        );
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        assert_eq!(
            next(&rx),
            LinkEvent::Received("cloud".to_string(), alloc::vec![0x05])
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "the second altered response is not told again, and the envelope arrives once"
        );
        client.stop();

        let asked = port.asked.lock().unwrap();
        assert!(asked.len() >= 4, "{} exchanges", asked.len());
        for request in &asked[..3] {
            assert_eq!(
                header_of(request, ACK_HEADER).as_deref(),
                Some("0"),
                "nothing is acknowledged before a response verified"
            );
        }
        assert_eq!(header_of(&asked[3], ACK_HEADER).as_deref(), Some("1"));
        let nonces: Vec<String> = asked
            .iter()
            .map(|request| header_of(request, NONCE_HEADER).expect("a nonce on every request"))
            .collect();
        assert_ne!(nonces[0], nonces[1], "a nonce is drawn for each exchange");
    }

    /// SCE_MESH.md section 22.9: a client of the `key` authenticator signs every request, a poll
    /// and a send alike, over its body's digest, sends no `Authorization`, and offers its public
    /// key. Red if a request leaves unsigned or its signature misses a covered field.
    #[test]
    fn a_client_of_the_key_authenticator_signs_every_request() {
        let key = KeyPair::from_seed(&[6u8; 32]);
        let port = Arc::new(Scripted::default());
        let signing = ClientSettings {
            credential: Credential::Key(RequestSigner::new(key.clone())),
            ..settings()
        };
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start(signing, port.clone(), move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        client.transport().transmit("cloud", &[0x09]).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !port
            .asked
            .lock()
            .unwrap()
            .iter()
            .any(|request| !request.body.is_empty())
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        client.stop();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let asked = port.asked.lock().unwrap();
        assert!(asked.iter().any(|request| !request.body.is_empty()));
        for request in asked.iter() {
            let field = |name: &str| field_value(&request.headers, name);
            assert_eq!(field("Authorization"), None);
            assert!(digest_matches(
                &field(CONTENT_DIGEST_HEADER).expect("a digest"),
                &request.body
            ));
            let target = normalize_target_uri(&request.url).expect("a target");
            let signature = RequestSignature::read(&field).expect("a signature of the profile");
            assert_eq!(
                signature.verify(&key.public_key(), &target, &field, now, 300),
                Ok(()),
                "{request:?}"
            );
        }
    }

    /// docs/adr/0109, D16: every exchange the client makes is reported by the status its server
    /// answered, and one that got no response as `none`. Red if `take_outcome` reports a failure
    /// at the transport under a status, or reports a `401` as anything but `401`.
    #[test]
    fn every_exchange_is_reported_by_the_status_its_server_answered() {
        use sce_rust_metrics::recording::Recording;

        let port = Arc::new(Scripted::default());
        {
            let mut polls = port.polls.lock().unwrap();
            polls.push_back(Ok(ok(None, Vec::new())));
            polls.push_back(Err(HttpsFailure::Connect));
            polls.push_back(Ok(status(401, &[("WWW-Authenticate", "Bearer")])));
            polls.push_back(Ok(status(502, &[])));
        }
        let recording = Arc::new(Recording::new());
        let (tx, rx) = mpsc::channel();
        let client = HttpsClient::start_observed(
            settings(),
            port.clone(),
            move |event| {
                let _ = tx.send(event);
            },
            Arc::clone(&recording) as Arc<dyn Metrics>,
        )
        .unwrap();
        assert_eq!(next(&rx), LinkEvent::Ready("cloud".to_string()));
        let counted = |status| recording.counted(Counter::ClientExchanges { status });
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while counted(ExchangeOutcome::Other) == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        client.stop();
        assert!(counted(ExchangeOutcome::Ok) >= 1);
        assert_eq!(counted(ExchangeOutcome::NoResponse), 1);
        assert_eq!(counted(ExchangeOutcome::Unauthorized), 1);
        assert_eq!(
            counted(ExchangeOutcome::Other),
            1,
            "a 502 of an intermediary"
        );
        assert_eq!(
            recording.counted_in(&sce_rust_metrics::SERVER_EXCHANGES),
            0,
            "a client reports no exchange of a server's"
        );
    }
}
