// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The dialing side of the HTTPS binding (SCE_MESH.md section 22): a machine that reaches one
//! server machine by exchanges it opens, through the port [`HttpsTransport`].
//!
//! Two threads, each with at most one exchange in flight. The poller holds an exchange open
//! (`Sce-Mesh-Hold-Ms`) so that what the server sends reaches the client as soon as it is made;
//! the sender carries what the core transmits as soon as it is transmitted, without waiting for a
//! held exchange to end. Both acknowledge what the client has received, and what either response
//! carries is handed to the core once, in mailbox order.
//!
//! The binding keeps no decision of the core's. What it hands its host is [`LinkEvent`]s: the
//! server is ready (from the first exchange answered `200`), bytes arrived, an envelope the
//! exchange could not deliver (handed back as [`LinkEvent::Unsent`], which the core sends again
//! under the binding's retry policy or raises as SCE_MESH.md section 16.7 row 2 or row 3), and
//! the server was lost (an exchange failed at the transport).

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use super::{
    is_epoch, is_path_segment, retry_after, split_sequence, Credential, HttpsFailure, HttpsRequest,
    HttpsResponse, HttpsTransport, LinkEvent, ACK_HEADER, CONTENT_TYPE, EPOCH_HEADER, FIRST_HEADER,
    HOLD_HEADER, PATH_PREFIX,
};
use crate::endpoint::{Transport, TransportFailure};
use crate::peers::{HttpsAuthenticator, Peer, PeerLink};
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
    /// How the client proves its name (section 22.5).
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
        }
    }

    /// The settings the generated peer table gives the machine `own` for `peer`, when this
    /// machine dials it over HTTPS ([`PeerLink::HttpsDial`]), proving its name with `credential`,
    /// which the host holds. `None` for any other link, and for a credential of another kind than
    /// the link's `authenticator`, which the server would refuse.
    pub fn from_peer(own: &str, peer: &Peer, credential: Credential) -> Option<ClientSettings> {
        let PeerLink::HttpsDial {
            url,
            authenticator,
            hold_ms,
        } = peer.link
        else {
            return None;
        };
        let fits = matches!(
            (authenticator, &credential),
            (HttpsAuthenticator::Bearer, Credential::Bearer(_))
                | (HttpsAuthenticator::Install, Credential::Install(_))
                | (HttpsAuthenticator::Mtls, Credential::Mtls)
        );
        fits.then(|| ClientSettings {
            hold: Duration::from_millis(u64::from(hold_ms)),
            ..ClientSettings::new(url, own, peer.name, credential)
        })
    }
}

/// Why a client could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The machine name is not a path segment as it stands (section 22.1).
    NotAPathSegment(String),
    /// The base is not an `https://` URL, or an `http://` one for a loopback host, or carries a
    /// query or a fragment.
    BadBase(String),
}

/// What the two threads share.
struct Shared {
    settings: ClientSettings,
    /// The URL every exchange posts to.
    url: String,
    /// The `Authorization` header, when the credential is one.
    authorization: Option<String>,
    port: Arc<dyn HttpsTransport>,
    state: Mutex<State>,
    /// Wakes the sender when the core transmits, and either thread when the client stops.
    wake: Condvar,
}

type EventSink = Box<dyn FnMut(LinkEvent) + Send>;

struct State {
    /// What the core transmitted and no exchange has carried yet, oldest first.
    outbox: VecDeque<Vec<u8>>,
    /// What every request acknowledges: the highest mailbox number handed to the core, in the
    /// mailbox incarnation the client last saw.
    mark: Mark,
    ready: bool,
    stopped: bool,
    /// Where the host is told what happened. Called with the state held, so that what the two
    /// threads observe reaches the host in the order it was observed.
    events: EventSink,
}

/// Where an acknowledgement stands (section 22.3): the highest mailbox number handed to the core,
/// and the incarnation of the mailbox it counts in, `None` before the first `200`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Mark {
    ack: u64,
    epoch: Option<String>,
}

const HELD: &str = "the client's state is never left mid-update: nothing in it panics while held";

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().expect(HELD)
    }

    fn request(&self, body: Vec<u8>, mark: &Mark, hold: Duration) -> HttpsRequest {
        let mut headers = alloc::vec![
            ("Content-Type".to_string(), CONTENT_TYPE.to_string()),
            (ACK_HEADER.to_string(), mark.ack.to_string()),
            (HOLD_HEADER.to_string(), hold.as_millis().to_string()),
        ];
        if let Some(epoch) = &mark.epoch {
            headers.push((EPOCH_HEADER.to_string(), epoch.clone()));
        }
        if let Some(authorization) = &self.authorization {
            headers.push(("Authorization".to_string(), authorization.clone()));
        }
        HttpsRequest {
            url: self.url.clone(),
            headers,
            body,
            timeout: hold.saturating_add(self.settings.exchange_timeout),
        }
    }
}

/// What an exchange's outcome asks of the thread that made it.
#[derive(Debug, PartialEq, Eq)]
enum Then {
    /// Go on at once.
    Continue,
    /// Wait at least this long first: the server asked (`Retry-After`) or the exchange failed.
    Wait(Duration),
}

/// Take what an exchange returned. `carried` are the envelopes the request carried and `sent` the
/// acknowledgement it sent. Returns what the thread should do next; a response the server shed
/// puts `carried` back at the front of the outbox, to go again after the wait.
fn take_outcome(
    shared: &Shared,
    carried: Vec<Vec<u8>>,
    sent: &Mark,
    outcome: Result<HttpsResponse, HttpsFailure>,
    failures: &mut u32,
) -> Then {
    let server = shared.settings.server.clone();
    let mut state = shared.lock();
    if state.stopped {
        // The client stopped while the exchange was in flight: it has told the host its server is
        // lost, and takes nothing more in. What the mailbox sent stays there, unacknowledged.
        for envelope in carried {
            (state.events)(LinkEvent::Unsent(server.clone(), envelope));
        }
        return Then::Continue;
    }
    match outcome {
        Ok(response) if response.status == 200 => {
            *failures = 0;
            if !state.ready {
                state.ready = true;
                (state.events)(LinkEvent::Ready(server.clone()));
            }
            take_mailbox(&mut state, &server, sent, &response);
            Then::Continue
        }
        Ok(response) if matches!(response.status, 429 | 503) => {
            for envelope in carried.into_iter().rev() {
                state.outbox.push_front(envelope);
            }
            let asked = response
                .header("Retry-After")
                .and_then(|value| retry_after(value, SystemTime::now()));
            let wait = asked.unwrap_or_else(|| shared.settings.backoff.delay(*failures));
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
        // A status the binding does not take in: the server answered, so the client stays as
        // ready as it was, and what the request carried was not taken (section 22.4).
        Ok(_) => {
            for envelope in carried {
                (state.events)(LinkEvent::Unsent(server.clone(), envelope));
            }
            let wait = shared.settings.backoff.delay(*failures);
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
        Err(failure) => {
            for envelope in carried {
                (state.events)(LinkEvent::Unsent(server.clone(), envelope));
            }
            if state.ready {
                state.ready = false;
                (state.events)(LinkEvent::Lost(server, failure.to_string()));
            }
            let wait = shared.settings.backoff.delay(*failures);
            *failures = failures.saturating_add(1);
            Then::Wait(wait)
        }
    }
}

/// Hand the core the envelopes of a `200` response, in mailbox order and each once, and move the
/// acknowledgement past them (section 22.3). `sent` is what the response's request acknowledged.
///
/// The response's incarnation (`Sce-Mesh-Epoch`) says which numbering its numbers count in:
///
/// * the one the client last saw: what is at or below the client's acknowledgement (which the
///   other thread's response may already have handed on) is passed over;
/// * another, which the request was sent in: an incarnation the client has left since, because
///   the other thread's response named a newer one. What is above the request's acknowledgement is
///   handed on, the section 10.5 window dropping what reached the core before, and the client's
///   acknowledgement stays with the newer incarnation;
/// * any other: a mailbox that numbers anew (a server that started again with its mailboxes
///   empty). The acknowledgement starts over at 0 in it, so all of the response is new.
///
/// A response that names no incarnation says nothing of what its numbers count in, and a body
/// without a first number cannot be acknowledged: what a server that breaks the binding sends is
/// not taken.
fn take_mailbox(state: &mut State, server: &str, sent: &Mark, response: &HttpsResponse) {
    let Some(epoch) = response
        .header(EPOCH_HEADER)
        .map(str::trim)
        .filter(|epoch| is_epoch(epoch))
    else {
        return;
    };
    let current = if state.mark.epoch.as_deref() == Some(epoch) {
        true
    } else if sent.epoch.as_deref() == Some(epoch) {
        false
    } else {
        state.mark = Mark {
            ack: 0,
            epoch: Some(epoch.to_string()),
        };
        true
    };
    let envelopes = split_sequence(&response.body);
    if envelopes.is_empty() {
        return;
    }
    let first = response
        .header(FIRST_HEADER)
        .and_then(|value| value.trim().parse::<u64>().ok());
    let Some(first) = first.filter(|first| *first >= 1) else {
        return;
    };
    let taken = if current { state.mark.ack } else { sent.ack };
    let last = first.saturating_add(envelopes.len() as u64 - 1);
    for (number, envelope) in (first..).zip(envelopes) {
        if number <= taken {
            continue;
        }
        (state.events)(LinkEvent::Received(server.to_string(), envelope));
    }
    if current {
        state.mark.ack = state.mark.ack.max(last);
    }
}

/// The core's side of the client: what it transmits waits for the sender's next exchange.
#[derive(Clone)]
pub struct HttpsClientTransport {
    shared: Arc<Shared>,
}

impl Transport for HttpsClientTransport {
    fn transmit(&mut self, peer: &str, bytes: &[u8]) -> Result<(), TransportFailure> {
        if peer != self.shared.settings.server {
            return Err(TransportFailure {
                retryable: false,
                message: Some(alloc::format!("no HTTPS binding to {peer}")),
                queue_full: None,
            });
        }
        let mut state = self.shared.lock();
        if state.stopped {
            return Err(TransportFailure {
                retryable: true,
                message: Some("the HTTPS client has stopped".to_string()),
                queue_full: None,
            });
        }
        state.outbox.push_back(bytes.to_vec());
        self.shared.wake.notify_all();
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
        if !is_path_segment(&settings.own) {
            return Err(ClientError::NotAPathSegment(settings.own));
        }
        let base = settings.base.trim_end_matches('/');
        if !base_is_allowed(base) {
            return Err(ClientError::BadBase(settings.base));
        }
        let url = alloc::format!("{base}{PATH_PREFIX}{}", settings.own);
        let shared = Arc::new(Shared {
            authorization: settings.credential.authorization(),
            url,
            settings,
            port,
            state: Mutex::new(State {
                outbox: VecDeque::new(),
                mark: Mark::default(),
                ready: false,
                stopped: false,
                events: Box::new(events),
            }),
            wake: Condvar::new(),
        });
        let poller = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || poll(&shared))
        };
        let sender = {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || send(&shared))
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

    /// Stop the client: no exchange is begun after this, what waits in the outbox is handed back
    /// as [`LinkEvent::Unsent`], and a ready server is lost. Returns once both threads ended, which
    /// is once the exchange each has in flight ended.
    pub fn stop(mut self) {
        self.signal_stop();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }

    fn signal_stop(&self) {
        let server = self.shared.settings.server.clone();
        let mut state = self.shared.lock();
        if state.stopped {
            return;
        }
        state.stopped = true;
        let left: Vec<Vec<u8>> = state.outbox.drain(..).collect();
        for envelope in left {
            (state.events)(LinkEvent::Unsent(server.clone(), envelope));
        }
        if state.ready {
            state.ready = false;
            (state.events)(LinkEvent::Lost(
                server,
                "the HTTPS client stopped".to_string(),
            ));
        }
        self.shared.wake.notify_all();
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
/// loopback host, and in either no query or fragment.
fn base_is_allowed(base: &str) -> bool {
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

/// Wait `wait`, or less if the client stops meanwhile. Returns whether it stopped.
fn pause(shared: &Shared, wait: Duration) -> bool {
    let state = shared.lock();
    if state.stopped {
        return true;
    }
    let (state, _) = shared
        .wake
        .wait_timeout_while(state, wait, |state| !state.stopped)
        .expect(HELD);
    state.stopped
}

/// The poller: one held exchange after another, carrying nothing.
fn poll(shared: &Shared) {
    let mut failures = 0u32;
    loop {
        let mark = {
            let state = shared.lock();
            if state.stopped {
                return;
            }
            state.mark.clone()
        };
        let outcome = shared
            .port
            .exchange(shared.request(Vec::new(), &mark, shared.settings.hold));
        if let Then::Wait(wait) = take_outcome(shared, Vec::new(), &mark, outcome, &mut failures) {
            if pause(shared, wait) {
                return;
            }
        }
    }
}

/// The sender: an exchange that asks for no hold whenever the outbox holds something.
fn send(shared: &Shared) {
    let mut failures = 0u32;
    loop {
        let (carried, mark) = {
            let state = shared.lock();
            let mut state = shared
                .wake
                .wait_while(state, |state| state.outbox.is_empty() && !state.stopped)
                .expect(HELD);
            if state.stopped {
                return;
            }
            let mut carried = Vec::new();
            let mut size = 0usize;
            while let Some(length) = state.outbox.front().map(Vec::len) {
                let grown = size.saturating_add(length);
                if !carried.is_empty() && grown > shared.settings.max_body_bytes {
                    break;
                }
                size = grown;
                if let Some(envelope) = state.outbox.pop_front() {
                    carried.push(envelope);
                }
            }
            (carried, state.mark.clone())
        };
        let body = super::join_sequence(carried.iter().map(Vec::as_slice));
        let outcome = shared
            .port
            .exchange(shared.request(body, &mark, Duration::ZERO));
        if let Then::Wait(wait) = take_outcome(shared, carried, &mark, outcome, &mut failures) {
            if pause(shared, wait) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Where a request's acknowledgement stood, in `epoch`.
    fn mark(ack: u64, epoch: Option<&str>) -> Mark {
        Mark {
            ack,
            epoch: epoch.map(str::to_string),
        }
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

    /// A bare state for the pure rules, its events into a channel.
    fn state() -> (State, mpsc::Receiver<LinkEvent>) {
        let (tx, rx) = mpsc::channel();
        (
            State {
                outbox: VecDeque::new(),
                mark: Mark::default(),
                ready: true,
                stopped: false,
                events: Box::new(move |event| {
                    let _ = tx.send(event);
                }),
            },
            rx,
        )
    }

    fn received(rx: &mpsc::Receiver<LinkEvent>) -> Vec<Vec<u8>> {
        rx.try_iter()
            .map(|event| match event {
                LinkEvent::Received(peer, bytes) => {
                    assert_eq!(peer, "cloud");
                    bytes
                }
                other => panic!("expected only arrivals, got {other:?}"),
            })
            .collect()
    }

    /// Three one-byte CBOR items (the unsigned integers 1, 2 and 3), which split as three.
    fn three() -> Vec<u8> {
        alloc::vec![0x01, 0x02, 0x03]
    }

    #[test]
    fn a_response_that_continues_the_numbering_is_taken_once() {
        let (mut state, rx) = state();
        state.mark = mark(4, Some(EPOCH));
        let sent = state.mark.clone();
        // The request acknowledged 4; the mailbox answers 5, 6, 7.
        take_mailbox(&mut state, "cloud", &sent, &ok(Some(5), three()));
        assert_eq!(received(&rx), [[1], [2], [3]]);
        assert_eq!(state.mark, mark(7, Some(EPOCH)));
        // The other thread's request also acknowledged 4 and was answered 5 to 8: only 8 is new.
        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok(Some(5), alloc::vec![0x01, 0x02, 0x03, 0x04]),
        );
        assert_eq!(received(&rx), [[4]]);
        assert_eq!(state.mark, mark(8, Some(EPOCH)));
    }

    /// A server that started again with its mailboxes empty numbers them from 1 in a new
    /// incarnation: the acknowledgement starts over in it, so what it sends is all new though its
    /// numbers are below what the client acknowledged. The other thread's response to a request
    /// sent in the earlier incarnation, answered in the new one, is taken from where the new
    /// numbering stands; one answered in the earlier incarnation is handed on above what its
    /// request acknowledged, and leaves the acknowledgement in the new one.
    #[test]
    fn a_new_incarnation_starts_the_acknowledgement_over() {
        let (mut state, rx) = state();
        state.mark = mark(57, Some(EARLIER));
        let sent = state.mark.clone();
        take_mailbox(&mut state, "cloud", &sent, &ok(Some(1), three()));
        assert_eq!(received(&rx), [[1], [2], [3]]);
        assert_eq!(state.mark, mark(3, Some(EPOCH)));

        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok(Some(1), alloc::vec![0x01, 0x02, 0x03, 0x04]),
        );
        assert_eq!(received(&rx), [[4]], "taken once in the new numbering");
        assert_eq!(state.mark, mark(4, Some(EPOCH)));

        take_mailbox(
            &mut state,
            "cloud",
            &sent,
            &ok_in(EARLIER, Some(57), alloc::vec![0x09, 0x0a]),
        );
        assert_eq!(
            received(&rx),
            [[0x0a]],
            "above what the request acknowledged"
        );
        assert_eq!(
            state.mark,
            mark(4, Some(EPOCH)),
            "the newer incarnation stays"
        );
    }

    #[test]
    fn a_body_without_a_number_or_an_incarnation_is_not_taken() {
        let (mut state, rx) = state();
        take_mailbox(&mut state, "cloud", &Mark::default(), &ok(None, three()));
        assert!(received(&rx).is_empty());
        assert_eq!(
            state.mark,
            mark(0, Some(EPOCH)),
            "the incarnation is learnt"
        );
        take_mailbox(&mut state, "cloud", &Mark::default(), &ok(Some(0), three()));
        assert!(received(&rx).is_empty(), "mailbox numbers start at 1");
        let unnamed = HttpsResponse {
            status: 200,
            headers: alloc::vec![(FIRST_HEADER.to_string(), "1".to_string())],
            body: three(),
        };
        take_mailbox(&mut state, "cloud", &Mark::default(), &unnamed);
        assert!(received(&rx).is_empty(), "no incarnation names the numbers");
        assert_eq!(state.mark, mark(0, Some(EPOCH)));
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
                responders: &["cloud"],
                deadline_ms: None,
                reply_events: &[],
                causal: None,
            },
            link: PeerLink::HttpsDial {
                url: "https://cal.example/api",
                authenticator,
                hold_ms: 9_000,
            },
        }
    }

    /// A peer table's dialing link gives the client its URL, its server's name and its hold, and
    /// only with a credential of the kind the link's authenticator names.
    #[test]
    fn a_dialing_link_gives_the_settings_and_takes_only_its_own_credential() {
        let peer = dialed(HttpsAuthenticator::Install);
        let settings = ClientSettings::from_peer("phone", &peer, Credential::Install([1; 32]))
            .expect("an installation secret for an `install` link");
        assert_eq!(settings.base, "https://cal.example/api");
        assert_eq!(settings.own, "phone");
        assert_eq!(settings.server, "cloud");
        assert_eq!(settings.hold, Duration::from_millis(9_000));
        assert!(ClientSettings::from_peer("phone", &peer, Credential::Mtls).is_none());
        let bearer = dialed(HttpsAuthenticator::Bearer);
        assert!(
            ClientSettings::from_peer("phone", &bearer, Credential::Bearer("t".into())).is_some()
        );
        let accepted = Peer {
            link: PeerLink::ZenohInbox,
            ..bearer
        };
        assert!(
            ClientSettings::from_peer("phone", &accepted, Credential::Bearer("t".into())).is_none()
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
        let bad_base = ClientSettings {
            base: "http://cal.example".to_string(),
            ..settings()
        };
        assert!(matches!(
            HttpsClient::start(bad_base, port, |_| {}),
            Err(ClientError::BadBase(_))
        ));
    }

    fn next(rx: &mpsc::Receiver<LinkEvent>) -> LinkEvent {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("a link event within five seconds")
    }

    /// The first exchange answered `200` makes the server ready, a transport failure loses it and
    /// hands back what the failed exchange carried, and the next `200` makes it ready again.
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
        assert!(matches!(next(&rx), LinkEvent::Lost(peer, _) if peer == "cloud"));
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
}
